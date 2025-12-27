# RustyDB v0.5.1 - Core Foundation Layer API Documentation

**Document Version:** 1.0
**Release:** v0.5.1
**Classification:** Enterprise Production Documentation
**Last Updated:** 2025-12-25

---

## Table of Contents

1. [Overview](#overview)
2. [Error Handling](#error-handling)
3. [Type Aliases](#type-aliases)
4. [Core Value Types](#core-value-types)
5. [Schema and Data Model](#schema-and-data-model)
6. [Transaction Types](#transaction-types)
7. [Core Traits](#core-traits)
8. [Configuration](#configuration)
9. [Resource Management](#resource-management)
10. [Event System](#event-system)
11. [Thread Safety and Concurrency](#thread-safety-and-concurrency)
12. [Best Practices](#best-practices)
13. [Migration Guide](#migration-guide)

---

## Overview

The Core Foundation Layer provides the fundamental building blocks for all RustyDB modules. It consists of three primary modules:

- **error.rs**: Unified error handling with comprehensive error types
- **common/mod.rs**: Shared types, traits, and interfaces
- **lib.rs**: Library entry point, module declarations, and public API

### Design Principles

1. **Type Safety**: Strong typing with minimal runtime overhead
2. **Error Transparency**: Detailed error contexts for debugging and monitoring
3. **Thread Safety**: All public types are `Send + Sync` where applicable
4. **Performance**: Zero-cost abstractions with minimal allocations
5. **Compatibility**: Oracle-like semantics where appropriate

### Module Dependencies

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Network, API, Query Processing)       │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│         Storage & Transaction Layer     │
│  (Storage, Buffer, Transaction, Index)  │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│         Core Foundation Layer           │
│      (error, common, lib.rs)            │
└─────────────────────────────────────────┘
```

All modules depend on the Core Foundation Layer for error types and common interfaces.

---

## Error Handling

### DbError Enum

The `DbError` enum is the unified error type used across all RustyDB modules. It uses the `thiserror` crate for automatic `Error` trait implementation and display formatting.

**Location:** `src/error.rs`

#### Type Signature

```rust
#[derive(Error, Debug)]
pub enum DbError { /* ... */ }

// Note: Clone is implemented manually (not derived) to handle Arc<std::io::Error>
impl Clone for DbError { /* ... */ }

pub type Result<T> = std::result::Result<T, DbError>;
```

#### Key Characteristics

- **Cloneable**: Manually implements `Clone` (not derived) for error propagation across threads
- **Thread-Safe**: `Send + Sync` for concurrent error handling
- **Arc-Wrapped I/O**: `std::io::Error` wrapped in `Arc` for cheap cloning
- **String Messages**: Most variants use `String` for detailed context

#### Complete Variant Reference

| Variant | Description | Use Case | Example |
|---------|-------------|----------|---------|
| `Io(Arc<std::io::Error>)` | I/O operation failures | File/network I/O errors | Disk read failure, connection timeout |
| `SqlParse(String)` | SQL parsing errors | Invalid SQL syntax | Malformed SELECT statement |
| `Transaction(String)` | Transaction management errors | Transaction lifecycle issues | Commit after rollback |
| `Storage(String)` | Storage layer errors | Page/disk operations | Page not found, disk full |
| `Catalog(String)` | Catalog/metadata errors | Schema operations | Table already exists |
| `Index(String)` | Index operation errors | Index maintenance | Index corruption detected |
| `Execution(String)` | Query execution errors | Runtime query errors | Division by zero |
| `Network(String)` | Network communication errors | Protocol/connection errors | Invalid packet format |
| `Serialization(String)` | Data serialization errors | Encoding/decoding failures | Invalid bincode data |
| `LockTimeout` | Lock acquisition timeout | Deadlock prevention | Lock wait timeout exceeded |
| `LockError(String)` | Lock management errors | Locking failures | Lock table overflow |
| `Unavailable(String)` | Service unavailable | System overload | Circuit breaker open |
| `Deadlock` | Deadlock detected | Transaction conflicts | Circular lock dependency |
| `NotFound(String)` | Resource not found | Missing entities | Table/row not found |
| `AlreadyExists(String)` | Resource already exists | Duplicate creation | Table name collision |
| `InvalidInput(String)` | Invalid user input | Validation failures | Invalid column type |
| `InvalidOperation(String)` | Invalid operation | Logic errors | Update on read-only view |
| `NotImplemented(String)` | Feature not implemented | Unsupported features | LATERAL joins |
| `Internal(String)` | Internal system errors | Unexpected conditions | Assertion failures |
| `Validation(String)` | Data validation errors | Constraint violations | Check constraint failed |
| `BackupError(String)` | Backup/restore errors | Backup operations | Corrupt backup file |
| `Runtime(String)` | Runtime errors | General runtime issues | Configuration error |
| `Replication(String)` | Replication errors | Cluster replication | Replication lag exceeded |
| `InvalidArgument(String)` | Invalid function argument | Parameter validation | Negative page size |
| `ResourceExhausted(String)` | Resource exhaustion | Quota/limit exceeded | Out of file descriptors |
| `BsonError(String)` | BSON operations | Document store errors | Invalid BSON format |
| `Encryption(String)` | Encryption/decryption errors | Security operations | Invalid key format |
| `OutOfMemory(String)` | Memory allocation failure | OOM conditions | Buffer pool exhausted |
| `LimitExceeded(String)` | Limit exceeded | Size/count limits | Max columns exceeded |
| `Configuration(String)` | Configuration errors | Config validation | Invalid port number |
| `PermissionDenied(String)` | Authorization failure | Security checks | Insufficient privileges |
| `Timeout(String)` | Operation timeout | Time limits | Query timeout |
| `Cluster(String)` | Cluster management errors | Distributed operations | Node unreachable |
| `Buffer(String)` | Buffer pool errors | Buffer management | No free frames |
| `Simd(String)` | SIMD operation errors | Vectorization errors | Unsupported CPU feature |
| `Concurrent(String)` | Concurrency errors | Parallel operations | CAS retry limit exceeded |
| `CircuitBreakerOpen(String)` | Circuit breaker tripped | Fault tolerance | Too many failures |
| `BulkheadFull(String)` | Bulkhead full | Resource isolation | Thread pool saturated |
| `Security(String)` | Security violations | Security subsystem | Policy violation |
| `InjectionAttempt(String)` | Injection attack detected | Security defenses | SQL injection blocked |
| `InvalidRequest` | Invalid request format | Protocol errors | Malformed request |
| `InvalidState(String)` | Invalid state transition | State machine errors | Already initialized |
| `QuotaExceeded(String)` | Quota exceeded | Resource quotas | Storage quota exceeded |
| `PageNotFound(String)` | Page not found | Storage layer | Invalid page ID |
| `Other(String)` | Catch-all errors | Uncategorized | Misc error |
| `Authentication(String)` | Authentication failure | Login/auth | Invalid credentials |
| `Authorization(String)` | Authorization failure | Access control | Role check failed |
| `Compression(String)` | Compression errors | Data compression | Decompression failed |
| `Recovery(String)` | Recovery errors | Crash recovery | WAL corruption |
| `Memory(String)` | Memory management errors | Allocator errors | Slab allocation failed |
| `Corruption(String)` | Data corruption detected | Integrity checks | Checksum mismatch |
| `Conflict(String)` | Conflict detected | Write conflicts | MVCC conflict |
| `ConstraintViolation(String)` | Constraint violation | Integrity constraints | Foreign key violation |
| `ParseError(String)` | Generic parsing errors | Data parsing | Invalid JSON |

#### Error Construction Patterns

**Direct Construction:**
```rust
use rusty_db::error::{DbError, Result};

// Simple error
return Err(DbError::NotFound("Table 'users' not found".to_string()));

// With context
return Err(DbError::Transaction(format!(
    "Cannot commit transaction {} - already rolled back",
    txn_id
)));
```

**From Trait Conversions:**
```rust
// Automatic conversion from std::io::Error
let file = std::fs::File::open("data.db")?; // Auto-converts to DbError::Io

// Automatic conversion from serialization errors
let data: MyStruct = bincode::deserialize(bytes)?; // Auto-converts to DbError::Serialization
```

**Helper Methods:**
```rust
impl DbError {
    pub(crate) fn not_supported(feature: String) -> DbError {
        DbError::NotImplemented(feature)
    }
}
```

#### Error Propagation

Use the `?` operator for idiomatic error propagation:

```rust
use rusty_db::Result;

pub fn execute_query(sql: &str) -> Result<Vec<Tuple>> {
    let plan = parse_sql(sql)?;           // Propagates DbError::SqlParse
    let data = fetch_data(&plan)?;         // Propagates DbError::Storage
    let result = apply_filters(data)?;     // Propagates DbError::Execution
    Ok(result)
}
```

#### Thread Safety

`DbError` is fully thread-safe (`Send + Sync`):

```rust
use std::sync::Arc;
use std::thread;

let error = Arc::new(DbError::LockTimeout);
let handles: Vec<_> = (0..4)
    .map(|_| {
        let err = Arc::clone(&error);
        thread::spawn(move || {
            // Error can be safely shared across threads
            log_error(&err);
        })
    })
    .collect();
```

#### Security Considerations

1. **Message Length Validation**: Use `common::validate_error_message()` to prevent unbounded string allocations
2. **Sensitive Data**: Never include passwords, tokens, or PII in error messages
3. **Injection Prevention**: `InjectionAttempt` variant flags potential security attacks
4. **Audit Logging**: Security-related errors trigger audit events

#### Performance Notes

- **Arc for I/O Errors**: `std::io::Error` wrapped in `Arc` for cheap cloning (single atomic increment)
- **String Allocations**: Most variants use `String` - consider using `Cow<'static, str>` for static messages (TODO)
- **Clone Cost**: O(1) for unit variants, O(n) for String variants where n is message length

---

## Type Aliases

### Identifier Types

**Location:** `src/common/mod.rs`

```rust
/// Unique identifier for transactions (64-bit unsigned integer)
pub type TransactionId = u64;

/// Unique identifier for pages in storage (64-bit unsigned integer)
pub type PageId = u64;

/// Unique identifier for tables in the catalog (32-bit unsigned integer)
pub type TableId = u32;

/// Unique identifier for indexes (32-bit unsigned integer)
pub type IndexId = u32;

/// Unique identifier for columns within a table (16-bit unsigned integer)
pub type ColumnId = u16;

/// Unique identifier for rows - physical location (64-bit unsigned integer)
pub type RowId = u64;

/// Log Sequence Number for write-ahead logging (64-bit unsigned integer)
pub type LogSequenceNumber = u64;

/// Node identifier in a cluster (String)
pub type NodeId = String;

/// Session identifier for user connections (64-bit unsigned integer)
pub type SessionId = u64;
```

### Usage Patterns

```rust
use rusty_db::common::{TransactionId, PageId, TableId};

// Type safety prevents mixing different ID types
fn load_page(table_id: TableId, page_id: PageId) -> Result<Page> {
    // compiler error: cannot pass TransactionId where PageId expected
}

// Consistent ID generation
let txn_id: TransactionId = self.next_txn_id.fetch_add(1, Ordering::SeqCst);
let page_id: PageId = calculate_page_id(table_id, offset);
```

### ID Range Limits

| Type | Size | Max Value | Notes |
|------|------|-----------|-------|
| `TransactionId` | 64-bit | 18,446,744,073,709,551,615 | ~584 billion years at 1M TPS |
| `PageId` | 64-bit | 18,446,744,073,709,551,615 | 128 exabytes with 8KB pages |
| `TableId` | 32-bit | 4,294,967,295 | 4.2 billion tables |
| `IndexId` | 32-bit | 4,294,967,295 | 4.2 billion indexes |
| `ColumnId` | 16-bit | 65,535 | Limited by MAX_COLUMNS_PER_TABLE (1024) |
| `RowId` | 64-bit | 18,446,744,073,709,551,615 | Physical row address |
| `SessionId` | 64-bit | 18,446,744,073,709,551,615 | Unique session tracking |

### NodeId Special Considerations

`NodeId` is a `String` (not numeric) to support flexible cluster addressing:
- Human-readable names: `"primary-us-west"`, `"replica-01"`
- UUID-based: `"550e8400-e29b-41d4-a716-446655440000"`
- IP-based: `"192.168.1.100:5432"`

---

## Core Value Types

### Value Enum

The `Value` enum represents all possible data values stored in the database.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Null,                      // SQL NULL
    Boolean(bool),             // true/false
    Integer(i64),              // 64-bit signed integer
    Float(f64),                // 64-bit floating point
    String(String),            // UTF-8 variable-length string
    Bytes(Vec<u8>),           // Binary data
    Date(i64),                 // Days since epoch
    Timestamp(i64),            // Microseconds since epoch
    Json(serde_json::Value),  // JSON document
    Array(Vec<Value>),         // Array of values
    Text,                      // Text type marker
}
```

#### Value Methods

```rust
impl Value {
    /// Check if value is NULL
    pub fn is_null(&self) -> bool;

    /// Get type name as string
    pub fn type_name(&self) -> &str;

    /// Convert to string for display
    pub fn to_display_string(&self) -> String;
}
```

#### Value Examples

```rust
use rusty_db::common::Value;

// Constructing values
let null_val = Value::Null;
let int_val = Value::Integer(42);
let str_val = Value::String("Alice".to_string());
let json_val = Value::Json(serde_json::json!({
    "name": "Bob",
    "age": 30
}));
let array_val = Value::Array(vec![
    Value::Integer(1),
    Value::Integer(2),
    Value::Integer(3),
]);

// Type checking
assert!(null_val.is_null());
assert_eq!(int_val.type_name(), "INTEGER");
assert_eq!(str_val.to_display_string(), "Alice");
```

#### Value Comparison

`Value` implements `PartialEq`, `Eq`, `PartialOrd`, and `Ord`:

```rust
// Equality
assert_eq!(Value::Integer(42), Value::Integer(42));
assert_ne!(Value::Integer(42), Value::String("42".to_string()));

// Ordering (NULL < all other values)
assert!(Value::Null < Value::Integer(0));
assert!(Value::Integer(1) < Value::Integer(2));
assert!(Value::String("a".to_string()) < Value::String("b".to_string()));
```

**Special Float Comparison:**
- NaN == NaN (unlike standard f64)
- NaN > all non-NaN values
- Uses bit-level equality for deterministic comparison

#### Value Hashing

`Value` implements `Hash` for use in hash-based data structures:

```rust
use std::collections::HashMap;
use rusty_db::common::Value;

let mut map = HashMap::new();
map.insert(Value::Integer(1), "one");
map.insert(Value::String("key".to_string()), "value");
```

**Hash Behavior:**
- Discriminant hashed first (ensures different variants hash differently)
- Float values: hashes `f64::to_bits()` for deterministic hashing
- JSON values: hashes string representation (expensive - avoid as hash keys)

#### Nesting Depth Limits

To prevent stack overflow, arrays have a maximum nesting depth:

```rust
use rusty_db::common::{Value, validate_value_nesting, MAX_VALUE_NESTING_DEPTH};

// Maximum depth is 32 levels
assert_eq!(MAX_VALUE_NESTING_DEPTH, 32);

// Validation function
let nested_array = Value::Array(vec![Value::Array(vec![Value::Integer(1)])]);
validate_value_nesting(&nested_array, 0)?; // OK

// Exceeding depth returns error
let too_deep = create_nested_array(33); // hypothetical
validate_value_nesting(&too_deep, 0)?; // Err(DbError::LimitExceeded)
```

---

## Schema and Data Model

### Tuple Struct

Represents a row of data with MVCC metadata.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tuple {
    pub values: Vec<Value>,              // Column values
    pub row_id: RowId,                   // Physical row identifier
    pub xmin: Option<TransactionId>,     // Creating transaction
    pub xmax: Option<TransactionId>,     // Deleting transaction
}
```

#### Tuple Construction

```rust
use rusty_db::common::{Tuple, Value};

// Unchecked construction (for compatibility)
let tuple = Tuple::new(vec![
    Value::Integer(1),
    Value::String("Alice".to_string()),
], 100);

// Checked construction with validation
let tuple = Tuple::new_checked(values, row_id)?;
// Validates:
// - values.len() <= MAX_TUPLE_VALUES (1024)
// - Each value nesting depth <= MAX_VALUE_NESTING_DEPTH (32)
```

#### MVCC Visibility

```rust
impl Tuple {
    /// Check if tuple is visible to a transaction
    pub fn is_visible(&self, txn_id: TransactionId, snapshot: &Snapshot) -> bool {
        match (self.xmin, self.xmax) {
            // Created by committed txn, not deleted
            (Some(xmin), None) => snapshot.is_visible(xmin),

            // Created and deleted
            (Some(xmin), Some(xmax)) =>
                snapshot.is_visible(xmin) && !snapshot.is_visible(xmax),

            _ => false,
        }
    }
}
```

**Visibility Rules:**
1. Tuple created by visible transaction → visible
2. Tuple deleted by visible transaction → invisible
3. Tuple created by in-progress transaction → invisible (to other transactions)

### Schema Struct

Database table schema definition.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Schema {
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_key: Option<Vec<ColumnId>>,
    pub foreign_keys: Vec<ForeignKeyConstraint>,
    pub unique_constraints: Vec<Vec<ColumnId>>,
}
```

#### Schema Construction

```rust
use rusty_db::common::{Schema, ColumnDef, DataType};

// Create schema
let columns = vec![
    ColumnDef::new("id".to_string(), DataType::Integer).not_null(),
    ColumnDef::new("name".to_string(), DataType::Varchar(255)),
    ColumnDef::new("email".to_string(), DataType::Varchar(255)),
];

let mut schema = Schema::new_checked("users".to_string(), columns)?;
// Validates: columns.len() <= MAX_COLUMNS_PER_TABLE (1024)

// Add primary key
schema.primary_key = Some(vec![0]); // id column

// Add unique constraint
schema.add_unique_constraint(vec![2])?; // email column
// Validates: unique_constraints.len() <= MAX_UNIQUE_CONSTRAINTS_PER_TABLE (256)
```

#### Column Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<Value>,
    pub auto_increment: bool,
}
```

**Builder Pattern:**
```rust
let column = ColumnDef::new("created_at".to_string(), DataType::Timestamp)
    .not_null()
    .with_default(Value::Timestamp(current_timestamp()));
```

#### Data Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Integer,              // 32-bit signed integer
    BigInt,               // 64-bit signed integer
    Float,                // 32-bit floating point
    Double,               // 64-bit floating point
    Varchar(usize),       // Variable-length string (max length)
    Text,                 // Unlimited text
    Boolean,              // true/false
    Date,                 // Date (no time)
    Timestamp,            // Timestamp with timezone
    Json,                 // JSON document
    Blob,                 // Binary large object
    Decimal(u8, u8),      // Decimal(precision, scale)
    Array(Box<DataType>), // Array of type
}
```

**Type Compatibility Matrix:**

| DataType | Storage Size | Value Variant | Notes |
|----------|--------------|---------------|-------|
| `Integer` | 4 bytes | `Value::Integer` | Stored as i64 |
| `BigInt` | 8 bytes | `Value::Integer` | Native i64 |
| `Float` | 4 bytes | `Value::Float` | Stored as f64 |
| `Double` | 8 bytes | `Value::Float` | Native f64 |
| `Varchar(n)` | Variable | `Value::String` | Max n bytes UTF-8 |
| `Text` | Variable | `Value::String` | Unlimited |
| `Boolean` | 1 byte | `Value::Boolean` | true/false |
| `Date` | 8 bytes | `Value::Date` | Days since epoch |
| `Timestamp` | 8 bytes | `Value::Timestamp` | Microseconds since epoch |
| `Json` | Variable | `Value::Json` | serde_json::Value |
| `Blob` | Variable | `Value::Bytes` | Binary data |
| `Decimal(p,s)` | Variable | `Value::String` | Stored as string (TODO) |
| `Array(T)` | Variable | `Value::Array` | Homogeneous arrays |

#### Foreign Key Constraints

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKeyConstraint {
    pub name: String,
    pub columns: Vec<ColumnId>,
    pub referenced_table: TableId,
    pub referenced_columns: Vec<ColumnId>,
    pub on_delete: ReferentialAction,
    pub on_update: ReferentialAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferentialAction {
    NoAction,    // No action (default)
    Cascade,     // Cascade delete/update
    SetNull,     // Set foreign key to NULL
    SetDefault,  // Set to default value
    Restrict,    // Prevent delete/update
}
```

**Example:**
```rust
let fk = ForeignKeyConstraint {
    name: "fk_user_orders".to_string(),
    columns: vec![0], // user_id column
    referenced_table: users_table_id,
    referenced_columns: vec![0], // id column in users table
    on_delete: ReferentialAction::Cascade,
    on_update: ReferentialAction::NoAction,
};

schema.add_foreign_key(fk)?;
// Validates: foreign_keys.len() <= MAX_FOREIGN_KEYS_PER_TABLE (256)
```

---

## Transaction Types

### IsolationLevel Enum

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,    // Dirty reads allowed
    ReadCommitted,      // Prevents dirty reads (default)
    RepeatableRead,     // Prevents dirty & non-repeatable reads
    Serializable,       // Full isolation (no anomalies)
    SnapshotIsolation,  // MVCC-based snapshot isolation
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::ReadCommitted
    }
}
```

#### Isolation Level Characteristics

| Level | Dirty Read | Non-Repeatable Read | Phantom Read | Implementation |
|-------|------------|---------------------|--------------|----------------|
| `ReadUncommitted` | Possible | Possible | Possible | No read locks |
| `ReadCommitted` | Prevented | Possible | Possible | Short read locks |
| `RepeatableRead` | Prevented | Prevented | Possible | Long read locks |
| `Serializable` | Prevented | Prevented | Prevented | Lock-based serialization |
| `SnapshotIsolation` | Prevented | Prevented | Prevented | MVCC (write conflicts possible) |

**Default:** `ReadCommitted` (Oracle-compatible default)

**Note:** `SnapshotIsolation` enum exists but is not yet distinct from `RepeatableRead` in the current implementation. Both use MVCC.

#### Usage

```rust
use rusty_db::common::IsolationLevel;
use rusty_db::transaction::TransactionManager;

let mut txn_mgr = TransactionManager::new();

// Use default (ReadCommitted)
let txn_id = txn_mgr.begin(IsolationLevel::default())?;

// Explicit isolation level
let txn_id = txn_mgr.begin(IsolationLevel::Serializable)?;
```

### Snapshot Struct

Transaction snapshot for MVCC visibility determination.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub snapshot_txn_id: TransactionId,      // Snapshot transaction ID
    pub active_txns: Vec<TransactionId>,     // Active at snapshot time
    pub min_active_txn: TransactionId,       // Minimum active TXN
    pub max_committed_txn: TransactionId,    // Maximum committed TXN
}
```

#### Snapshot Construction

```rust
use rusty_db::common::{Snapshot, MAX_ACTIVE_TRANSACTIONS};

let snapshot = Snapshot::new(
    100,              // snapshot_txn_id
    vec![98, 99],     // active_txns
    98,               // min_active_txn
    97,               // max_committed_txn
)?;
// Validates: active_txns.len() <= MAX_ACTIVE_TRANSACTIONS (100,000)
```

#### Visibility Rules

```rust
impl Snapshot {
    /// Check if a transaction is visible in this snapshot
    pub fn is_visible(&self, txn_id: TransactionId) -> bool {
        // Visible if:
        // 1. Committed before snapshot (txn_id < snapshot_txn_id)
        // 2. Not in active list
        txn_id < self.snapshot_txn_id
            && !self.active_txns.contains(&txn_id)
    }
}
```

**Visibility Algorithm:**
```
┌─────────────────────────────────────────────────────────┐
│ Timeline: ───[97]──[98]──[99]──[100(snapshot)]──[101]── │
│                    │    │       │                         │
│                    └────┴───────┘                         │
│                    active_txns                            │
│                                                           │
│ Visible:     [1..97]  ✓                                  │
│ Invisible:   [98,99]  ✗ (active)                         │
│ Invisible:   [100+]   ✗ (after snapshot)                 │
└─────────────────────────────────────────────────────────┘
```

### Lock Types

#### LockMode Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockMode {
    Shared,                // S - Read lock
    Exclusive,             // X - Write lock
    IntentShared,          // IS - Intent to read
    IntentExclusive,       // IX - Intent to write
    SharedIntentExclusive, // SIX - Read + intent to write
    Update,                // U - Update lock (upgrade to X)
}
```

**Lock Compatibility Matrix:**

```
              IS   IX   S    SIX  U    X
          ┌────────────────────────────┐
       IS │ ✓    ✓    ✓    ✓    ✓    ✗ │
       IX │ ✓    ✓    ✗    ✗    ✗    ✗ │
        S │ ✓    ✗    ✓    ✗    ✓    ✗ │
      SIX │ ✓    ✗    ✗    ✗    ✗    ✗ │
        U │ ✓    ✗    ✓    ✗    ✗    ✗ │
        X │ ✗    ✗    ✗    ✗    ✗    ✗ │
          └────────────────────────────┘
```

**Lock Strength (weakest to strongest):**
1. `IntentShared` (IS)
2. `IntentExclusive` (IX)
3. `Update` (U)
4. `Shared` (S)
5. `SharedIntentExclusive` (SIX)
6. `Exclusive` (X)

#### LockResource Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockResource {
    Table(TableId),           // Table-level lock
    Page(PageId),             // Page-level lock
    Row(TableId, RowId),      // Row-level lock
    Database,                 // Database-level lock
}
```

**Lock Granularity Hierarchy:**
```
Database
 └─ Table
     └─ Page
         └─ Row
```

**Intent Locks:** Required on all ancestors when locking descendants.

**Example:**
```rust
// To lock a row with X lock:
// 1. Acquire IX lock on Database
// 2. Acquire IX lock on Table
// 3. Acquire IX lock on Page
// 4. Acquire X lock on Row
```

---

## Core Traits

### Component Trait

Base lifecycle trait for all major components.

**Location:** `src/common/mod.rs`

```rust
pub trait Component: Send + Sync {
    /// Initialize the component
    fn initialize(&mut self) -> Result<()>;

    /// Shutdown the component gracefully
    fn shutdown(&mut self) -> Result<()>;

    /// Check health status
    fn health_check(&self) -> HealthStatus;
}
```

#### Implementation Example

```rust
use rusty_db::common::{Component, HealthStatus};
use rusty_db::Result;

struct BufferPoolManager {
    initialized: bool,
    // ... fields
}

impl Component for BufferPoolManager {
    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Err(DbError::InvalidState(
                "BufferPool already initialized".to_string()
            ));
        }

        // Allocate buffer frames
        self.allocate_frames()?;

        // Start background flush thread
        self.start_flush_worker()?;

        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Flush dirty pages
        self.flush_all_dirty_pages()?;

        // Stop background threads
        self.stop_flush_worker()?;

        // Release resources
        self.free_frames()?;

        self.initialized = false;
        Ok(())
    }

    fn health_check(&self) -> HealthStatus {
        if !self.initialized {
            return HealthStatus::Unhealthy;
        }

        let dirty_ratio = self.dirty_page_count() as f64 / self.total_frames as f64;
        if dirty_ratio > 0.9 {
            HealthStatus::Degraded // High dirty page ratio
        } else {
            HealthStatus::Healthy
        }
    }
}
```

#### HealthStatus Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,    // Fully operational
    Degraded,   // Operational but degraded performance
    Unhealthy,  // Not operational
    Unknown,    // State unknown
}
```

**Health Check Guidelines:**
- `Healthy`: Component operating within normal parameters
- `Degraded`: Component functional but performance/capacity reduced
- `Unhealthy`: Component cannot fulfill its responsibilities
- `Unknown`: Unable to determine state (initialization pending, etc.)

### Transactional Trait

Transaction-aware components.

**Location:** `src/common/mod.rs`

```rust
pub trait Transactional: Component {
    /// Begin a new transaction
    fn begin_transaction(&mut self, isolation: IsolationLevel) -> Result<TransactionId>;

    /// Commit a transaction
    fn commit(&mut self, txn_id: TransactionId) -> Result<()>;

    /// Rollback a transaction
    fn rollback(&mut self, txn_id: TransactionId) -> Result<()>;
}
```

#### Implementation Example

```rust
use rusty_db::common::{Transactional, IsolationLevel, TransactionId};

impl Transactional for TransactionManager {
    fn begin_transaction(&mut self, isolation: IsolationLevel) -> Result<TransactionId> {
        // Allocate new transaction ID
        let txn_id = self.next_txn_id.fetch_add(1, Ordering::SeqCst);

        // Create snapshot for MVCC
        let snapshot = self.create_snapshot(txn_id)?;

        // Create transaction state
        let txn_state = TransactionState {
            txn_id,
            isolation,
            snapshot,
            status: TxnStatus::Active,
            start_time: SystemTime::now(),
        };

        self.active_transactions.insert(txn_id, txn_state);

        // Emit event
        self.emit_event(SystemEvent::TransactionBegin { txn_id, isolation })?;

        Ok(txn_id)
    }

    fn commit(&mut self, txn_id: TransactionId) -> Result<()> {
        let mut txn = self.active_transactions.get_mut(&txn_id)
            .ok_or_else(|| DbError::NotFound(format!("Transaction {} not found", txn_id)))?;

        // Validate state
        if txn.status != TxnStatus::Active {
            return Err(DbError::InvalidOperation(
                format!("Cannot commit transaction {} - status: {:?}", txn_id, txn.status)
            ));
        }

        // Write commit record to WAL
        self.wal.write_commit_record(txn_id)?;

        // Release locks
        self.lock_manager.release_all_locks(txn_id)?;

        // Update status
        txn.status = TxnStatus::Committed;

        // Emit event
        self.emit_event(SystemEvent::TransactionCommit { txn_id })?;

        Ok(())
    }

    fn rollback(&mut self, txn_id: TransactionId) -> Result<()> {
        // Similar to commit but undo changes instead
        // ...
        Ok(())
    }
}
```

### Recoverable Trait

Components supporting crash recovery.

**Location:** `src/common/mod.rs`

```rust
pub trait Recoverable: Component {
    /// Create a checkpoint
    fn checkpoint(&self) -> Result<()>;

    /// Recover from a specific log sequence number
    fn recover(&mut self, lsn: LogSequenceNumber) -> Result<()>;
}
```

#### Implementation Example

```rust
use rusty_db::common::{Recoverable, LogSequenceNumber};

impl Recoverable for StorageManager {
    fn checkpoint(&self) -> Result<()> {
        // Get current LSN
        let checkpoint_lsn = self.wal.current_lsn();

        // Flush all dirty pages to disk
        self.buffer_pool.flush_all_dirty_pages()?;

        // Write checkpoint record to WAL
        self.wal.write_checkpoint_record(checkpoint_lsn)?;

        // Truncate old WAL files
        self.wal.truncate_before(checkpoint_lsn)?;

        // Emit event
        self.emit_event(SystemEvent::CheckpointCompleted { lsn: checkpoint_lsn })?;

        Ok(())
    }

    fn recover(&mut self, from_lsn: LogSequenceNumber) -> Result<()> {
        // Read WAL from from_lsn to end
        let records = self.wal.read_from(from_lsn)?;

        // Redo phase: replay committed transactions
        for record in &records {
            match record {
                WalRecord::Insert { page_id, offset, data, txn_id } => {
                    if self.is_committed(*txn_id) {
                        self.redo_insert(*page_id, *offset, data)?;
                    }
                }
                WalRecord::Commit { txn_id } => {
                    self.mark_committed(*txn_id);
                }
                // ... other record types
                _ => {}
            }
        }

        // Undo phase: rollback incomplete transactions
        for record in records.iter().rev() {
            // Undo uncommitted changes
            // ...
        }

        Ok(())
    }
}
```

### Monitorable Trait

Components with metrics and observability.

**Location:** `src/common/mod.rs`

```rust
pub trait Monitorable: Component {
    /// Collect current metrics
    fn collect_metrics(&self) -> HashMap<String, MetricValue>;

    /// Get statistics
    fn get_statistics(&self) -> ComponentStatistics;
}
```

#### MetricValue Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),                          // Monotonic counter
    Gauge(f64),                            // Point-in-time value
    Histogram(Vec<f64>),                   // Distribution of values
    Summary { count: u64, sum: f64, min: f64, max: f64 }, // Summary stats
}
```

#### ComponentStatistics Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatistics {
    pub component_name: String,
    pub uptime: Duration,
    pub total_operations: u64,
    pub failed_operations: u64,
    pub avg_latency_ms: f64,
    pub custom_metrics: HashMap<String, MetricValue>,
}
```

**Size Limit:** Maximum 1,000 custom metrics per component (`MAX_CUSTOM_METRICS`)

#### Implementation Example

```rust
use std::collections::HashMap;
use rusty_db::common::{Monitorable, MetricValue, ComponentStatistics};

impl Monitorable for BufferPoolManager {
    fn collect_metrics(&self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        metrics.insert(
            "buffer_pool_hits".to_string(),
            MetricValue::Counter(self.cache_hits.load(Ordering::Relaxed))
        );

        metrics.insert(
            "buffer_pool_misses".to_string(),
            MetricValue::Counter(self.cache_misses.load(Ordering::Relaxed))
        );

        metrics.insert(
            "dirty_pages".to_string(),
            MetricValue::Gauge(self.dirty_page_count() as f64)
        );

        metrics.insert(
            "pin_latency_us".to_string(),
            MetricValue::Histogram(self.pin_latencies.clone())
        );

        metrics
    }

    fn get_statistics(&self) -> ComponentStatistics {
        let mut stats = ComponentStatistics::new("BufferPoolManager".to_string());

        stats.uptime = self.start_time.elapsed().unwrap_or_default();
        stats.total_operations = self.total_pins.load(Ordering::Relaxed);
        stats.failed_operations = self.failed_pins.load(Ordering::Relaxed);

        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let total = (self.cache_hits.load(Ordering::Relaxed)
                     + self.cache_misses.load(Ordering::Relaxed)) as f64;
        let hit_rate = if total > 0.0 { hits / total } else { 0.0 };

        stats.add_custom_metric(
            "cache_hit_rate".to_string(),
            MetricValue::Gauge(hit_rate)
        ).ok();

        stats
    }
}
```

### ReplicableState Trait

Serializable state for replication.

**Location:** `src/common/mod.rs`

```rust
pub trait ReplicableState: Component {
    /// Serialize component state
    fn serialize_state(&self) -> Result<Vec<u8>>;

    /// Deserialize and apply state
    fn deserialize_state(&mut self, data: &[u8]) -> Result<()>;

    /// Get current state version
    fn state_version(&self) -> u64;
}
```

#### Implementation Example

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct CatalogState {
    tables: HashMap<TableId, TableMetadata>,
    indexes: HashMap<IndexId, IndexMetadata>,
    version: u64,
}

impl ReplicableState for CatalogManager {
    fn serialize_state(&self) -> Result<Vec<u8>> {
        let state = CatalogState {
            tables: self.tables.clone(),
            indexes: self.indexes.clone(),
            version: self.state_version.load(Ordering::SeqCst),
        };

        bincode::encode_to_vec(&state, bincode::config::standard())
            .map_err(|e| DbError::Serialization(e.to_string()))
    }

    fn deserialize_state(&mut self, data: &[u8]) -> Result<()> {
        let (state, _): (CatalogState, usize) =
            bincode::decode_from_slice(data, bincode::config::standard())
                .map_err(|e| DbError::Serialization(e.to_string()))?;

        self.tables = state.tables;
        self.indexes = state.indexes;
        self.state_version.store(state.version, Ordering::SeqCst);

        Ok(())
    }

    fn state_version(&self) -> u64 {
        self.state_version.load(Ordering::SeqCst)
    }
}
```

---

## Configuration

### DatabaseConfig Struct

Global database configuration (current/recommended).

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    // Storage configuration
    pub data_dir: String,
    pub page_size: usize,
    pub buffer_pool_size: usize,
    pub wal_dir: String,
    pub checkpoint_interval: Duration,

    // Transaction configuration
    pub default_isolation: IsolationLevel,
    pub lock_timeout: Duration,
    pub deadlock_detection_interval: Duration,

    // Network configuration
    pub listen_address: String,
    pub port: u16,
    pub api_port: u16,
    pub enable_rest_api: bool,
    pub max_connections: usize,
    pub connection_timeout: Duration,

    // Security configuration
    pub enable_tls: bool,
    pub enable_encryption: bool,
    pub password_min_length: usize,
    pub session_timeout: Duration,

    // Clustering configuration
    pub cluster_enabled: bool,
    pub node_id: String,
    pub seed_nodes: Vec<String>,
    pub replication_factor: usize,

    // Performance configuration
    pub worker_threads: usize,
    pub enable_jit: bool,
    pub enable_vectorization: bool,
    pub query_timeout: Option<Duration>,
    pub max_memory_mb: usize,

    // Monitoring configuration
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub slow_query_threshold: Duration,
}
```

#### Default Configuration

```rust
impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            // Storage
            data_dir: "./data".to_string(),
            page_size: 8192,                              // 8 KB pages
            buffer_pool_size: 1000,                       // ~8 MB buffer pool
            wal_dir: "./wal".to_string(),
            checkpoint_interval: Duration::from_secs(300), // 5 minutes

            // Transaction
            default_isolation: IsolationLevel::ReadCommitted,
            lock_timeout: Duration::from_secs(30),
            deadlock_detection_interval: Duration::from_secs(1),

            // Network
            listen_address: "127.0.0.1".to_string(),
            port: 5432,                                    // PostgreSQL-compatible
            api_port: 8080,                                // REST API
            enable_rest_api: true,
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),

            // Security
            enable_tls: true,
            enable_encryption: true,
            password_min_length: 8,
            session_timeout: Duration::from_secs(3600),    // 1 hour

            // Clustering
            cluster_enabled: false,
            node_id: "node1".to_string(),
            seed_nodes: Vec::new(),
            replication_factor: 3,

            // Performance
            worker_threads: num_cpus(),                    // Auto-detect
            enable_jit: false,                             // JIT disabled by default
            enable_vectorization: true,                    // SIMD enabled
            query_timeout: Some(Duration::from_secs(300)), // 5 minutes
            max_memory_mb: 4096,                           // 4 GB

            // Monitoring
            enable_metrics: true,
            metrics_port: 9090,                            // Prometheus
            slow_query_threshold: Duration::from_millis(1000), // 1 second
        }
    }
}
```

#### Configuration Usage

```rust
use rusty_db::common::DatabaseConfig;

// Use defaults
let config = DatabaseConfig::default();

// Customize
let mut config = DatabaseConfig::default();
config.buffer_pool_size = 10000;  // 80 MB buffer pool
config.max_connections = 500;
config.worker_threads = 16;

// Production configuration
let prod_config = DatabaseConfig {
    data_dir: "/var/lib/rustydb".to_string(),
    buffer_pool_size: 100_000,  // ~800 MB
    max_connections: 1000,
    enable_tls: true,
    enable_encryption: true,
    cluster_enabled: true,
    seed_nodes: vec![
        "node1.example.com:5432".to_string(),
        "node2.example.com:5432".to_string(),
    ],
    replication_factor: 3,
    ..Default::default()
};
```

### Config Struct (Deprecated)

**Location:** `src/lib.rs`

```rust
#[deprecated(since = "0.1.0", note = "Use common::DatabaseConfig instead")]
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: String,
    pub page_size: usize,
    pub buffer_pool_size: usize,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            page_size: 4096,        // 4 KB (old default)
            buffer_pool_size: 1000, // ~4 MB
            port: 5432,
        }
    }
}
```

**Migration:** Use `common::DatabaseConfig` instead for new code. This struct is kept for backward compatibility only.

---

## Resource Management

### ResourceLimits Struct

Per-query and per-connection resource limits.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_bytes: usize,
    pub max_cpu_time: Duration,
    pub max_io_operations: usize,
    pub max_temp_space_bytes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024,      // 1 GB
            max_cpu_time: Duration::from_secs(300),     // 5 minutes
            max_io_operations: 1_000_000,               // 1M I/O ops
            max_temp_space_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
        }
    }
}
```

### ResourceUsage Struct

Track actual resource consumption.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub memory_bytes: usize,
    pub cpu_time: Duration,
    pub io_operations: usize,
    pub temp_space_bytes: usize,
    pub start_time: Option<SystemTime>,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            start_time: Some(SystemTime::now()),
            ..Default::default()
        }
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.and_then(|start| start.elapsed().ok())
    }
}
```

#### Usage Example

```rust
use rusty_db::common::{ResourceLimits, ResourceUsage};

// Set limits for a query
let limits = ResourceLimits {
    max_memory_bytes: 512 * 1024 * 1024, // 512 MB
    max_cpu_time: Duration::from_secs(60), // 1 minute
    ..Default::default()
};

// Track usage
let mut usage = ResourceUsage::new();

// During query execution
usage.memory_bytes += allocated_size;
usage.io_operations += 1;

// Check against limits
if usage.memory_bytes > limits.max_memory_bytes {
    return Err(DbError::ResourceExhausted(
        format!("Memory limit exceeded: {} bytes", usage.memory_bytes)
    ));
}

if usage.elapsed().unwrap_or_default() > limits.max_cpu_time {
    return Err(DbError::Timeout(
        format!("CPU time limit exceeded")
    ));
}
```

### Collection Size Limits

**Location:** `src/common/mod.rs`

```rust
/// Maximum number of columns per table
pub const MAX_COLUMNS_PER_TABLE: usize = 1024;

/// Maximum number of values in a tuple
pub const MAX_TUPLE_VALUES: usize = MAX_COLUMNS_PER_TABLE;

/// Maximum number of foreign keys per table
pub const MAX_FOREIGN_KEYS_PER_TABLE: usize = 256;

/// Maximum number of unique constraints per table
pub const MAX_UNIQUE_CONSTRAINTS_PER_TABLE: usize = 256;

/// Maximum number of active transactions in a snapshot
pub const MAX_ACTIVE_TRANSACTIONS: usize = 100_000;

/// Maximum number of custom metrics per component
pub const MAX_CUSTOM_METRICS: usize = 1_000;

/// Maximum depth of nested Value::Array
pub const MAX_VALUE_NESTING_DEPTH: usize = 32;

/// Maximum size of error message strings
pub const MAX_ERROR_MESSAGE_LENGTH: usize = 4096;
```

#### Validation Functions

```rust
/// Validate that a collection does not exceed size limits
pub fn validate_collection_size(
    collection_name: &str,
    actual_size: usize,
    max_size: usize,
) -> Result<()>;

/// Validate that error message length is within bounds
pub fn validate_error_message(message: &str) -> String;

/// Validate Value nesting depth to prevent stack overflow
pub fn validate_value_nesting(value: &Value, current_depth: usize) -> Result<()>;
```

**Purpose:** Prevent DoS attacks via unbounded memory allocation.

---

## Event System

### SystemEvent Enum

System-wide events for inter-module communication.

**Location:** `src/common/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    // Transaction events
    TransactionBegin { txn_id: TransactionId, isolation: IsolationLevel },
    TransactionCommit { txn_id: TransactionId },
    TransactionRollback { txn_id: TransactionId },

    // Storage events
    PageEvicted { page_id: PageId },
    CheckpointStarted { lsn: LogSequenceNumber },
    CheckpointCompleted { lsn: LogSequenceNumber },

    // Cluster events
    NodeJoined { node_id: NodeId },
    NodeLeft { node_id: NodeId },
    LeaderElected { node_id: NodeId },

    // Security events
    UserLogin { username: String, session_id: SessionId },
    UserLogout { session_id: SessionId },
    AuthenticationFailed { username: String, reason: String },
    PermissionDenied { user: String, resource: String },

    // Performance events
    SlowQuery { sql: String, duration: Duration },
    ResourceThresholdExceeded { resource: String, value: f64 },

    // Backup events
    BackupStarted { backup_id: String },
    BackupCompleted { backup_id: String },
    RestoreStarted { backup_id: String },
    RestoreCompleted { backup_id: String },
}
```

### EventListener Trait

**Location:** `src/common/mod.rs`

```rust
pub trait EventListener: Send + Sync {
    /// Handle a system event
    fn on_event(&mut self, event: SystemEvent) -> Result<()>;
}
```

#### Implementation Example

```rust
use rusty_db::common::{EventListener, SystemEvent};

struct AuditLogger {
    log_file: File,
}

impl EventListener for AuditLogger {
    fn on_event(&mut self, event: SystemEvent) -> Result<()> {
        match event {
            SystemEvent::UserLogin { username, session_id } => {
                writeln!(
                    self.log_file,
                    "[{}] User '{}' logged in (session: {})",
                    SystemTime::now(),
                    username,
                    session_id
                )?;
            }

            SystemEvent::PermissionDenied { user, resource } => {
                writeln!(
                    self.log_file,
                    "[{}] SECURITY: User '{}' denied access to '{}'",
                    SystemTime::now(),
                    user,
                    resource
                )?;
            }

            SystemEvent::SlowQuery { sql, duration } => {
                writeln!(
                    self.log_file,
                    "[{}] SLOW QUERY ({:?}): {}",
                    SystemTime::now(),
                    duration,
                    sql
                )?;
            }

            _ => {} // Ignore other events
        }

        Ok(())
    }
}
```

---

## Thread Safety and Concurrency

### Thread Safety Guarantees

All core types are designed for concurrent use:

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `DbError` | ✓ | ✓ | Arc-wrapped I/O errors |
| `Value` | ✓ | ✓ | Immutable after creation |
| `Tuple` | ✓ | ✓ | Immutable after creation |
| `Schema` | ✓ | ✓ | Immutable after creation |
| `IsolationLevel` | ✓ | ✓ | Copy type |
| `Snapshot` | ✓ | ✓ | Immutable after creation |
| `LockMode` | ✓ | ✓ | Copy type |
| `HealthStatus` | ✓ | ✓ | Copy type |
| `DatabaseConfig` | ✓ | ✓ | Cloneable |
| `ResourceLimits` | ✓ | ✓ | Cloneable |

### Trait Requirements

All core traits require `Send + Sync`:

```rust
pub trait Component: Send + Sync { /* ... */ }
pub trait Transactional: Component { /* ... */ }  // Inherits Send + Sync
pub trait Recoverable: Component { /* ... */ }    // Inherits Send + Sync
pub trait Monitorable: Component { /* ... */ }    // Inherits Send + Sync
pub trait ReplicableState: Component { /* ... */ } // Inherits Send + Sync
pub trait EventListener: Send + Sync { /* ... */ }
```

### Concurrency Patterns

**Arc for Shared Ownership:**
```rust
use std::sync::Arc;

let config = Arc::new(DatabaseConfig::default());
let config_clone = Arc::clone(&config);

// Share across threads
thread::spawn(move || {
    println!("Port: {}", config_clone.port);
});
```

**Mutex for Mutable State:**
```rust
use std::sync::{Arc, Mutex};

let shared_state = Arc::new(Mutex::new(MyComponent::new()));

thread::spawn({
    let state = Arc::clone(&shared_state);
    move || {
        let mut guard = state.lock().unwrap();
        guard.update()?;
    }
});
```

**RwLock for Read-Heavy Workloads:**
```rust
use std::sync::{Arc, RwLock};

let catalog = Arc::new(RwLock::new(CatalogManager::new()));

// Multiple readers
let reader_handle = thread::spawn({
    let catalog = Arc::clone(&catalog);
    move || {
        let catalog_read = catalog.read().unwrap();
        catalog_read.get_table("users")?;
    }
});

// Single writer
let writer_handle = thread::spawn({
    let catalog = Arc::clone(&catalog);
    move || {
        let mut catalog_write = catalog.write().unwrap();
        catalog_write.create_table(schema)?;
    }
});
```

---

## Best Practices

### Error Handling

**DO:**
```rust
// Use ? operator for propagation
fn my_operation() -> Result<Data> {
    let connection = establish_connection()?;
    let data = fetch_data(&connection)?;
    Ok(data)
}

// Provide context in errors
return Err(DbError::NotFound(format!(
    "Table '{}' not found in schema '{}'",
    table_name, schema_name
)));

// Validate inputs early
if table_name.is_empty() {
    return Err(DbError::InvalidInput("Table name cannot be empty".to_string()));
}
```

**DON'T:**
```rust
// Don't unwrap() in library code
let data = fetch_data().unwrap(); // ❌ Will panic

// Don't use generic error messages
return Err(DbError::Internal("Error".to_string())); // ❌ Not helpful

// Don't ignore errors
let _ = critical_operation(); // ❌ Silent failure
```

### Type Safety

**DO:**
```rust
// Use type aliases for clarity
fn get_transaction_status(txn_id: TransactionId) -> Result<TxnStatus>;

// Leverage newtype pattern for additional safety
struct PageOffset(u32);
struct SlotId(u16);

// Use enums for exhaustive matching
match isolation_level {
    IsolationLevel::ReadCommitted => { /* ... */ }
    IsolationLevel::Serializable => { /* ... */ }
    // Compiler ensures all cases handled
}
```

**DON'T:**
```rust
// Don't use raw numbers for IDs
fn get_page(id: u64) -> Result<Page>; // ❌ Is this PageId or TransactionId?

// Don't use strings for enums
fn set_isolation(level: &str) -> Result<()>; // ❌ Use IsolationLevel enum
```

### Resource Management

**DO:**
```rust
// Use validated constructors
let tuple = Tuple::new_checked(values, row_id)?; // ✓ Validates limits

let schema = Schema::new_checked(table_name, columns)?; // ✓ Validates limits

// Check resource limits proactively
validate_collection_size("columns", columns.len(), MAX_COLUMNS_PER_TABLE)?;

// Use RAII for cleanup
struct BufferGuard<'a> {
    buffer: &'a mut Buffer,
}

impl Drop for BufferGuard<'_> {
    fn drop(&mut self) {
        self.buffer.release();
    }
}
```

**DON'T:**
```rust
// Don't use unchecked constructors for user input
let tuple = Tuple::new(user_provided_values, row_id); // ❌ No validation

// Don't accumulate unbounded collections
let mut errors = Vec::new(); // ❌ Can grow without limit
for item in items {
    errors.push(process_item(item));
}
```

### Configuration

**DO:**
```rust
// Start with defaults, override as needed
let mut config = DatabaseConfig::default();
config.buffer_pool_size = 50_000;
config.max_connections = 500;

// Validate configuration
if config.buffer_pool_size == 0 {
    return Err(DbError::Configuration("Buffer pool size must be > 0".to_string()));
}

// Use environment-specific configs
let config = if cfg!(debug_assertions) {
    DatabaseConfig::default() // Development defaults
} else {
    load_production_config()? // Production tuning
};
```

### Component Lifecycle

**DO:**
```rust
// Always initialize before use
let mut component = MyComponent::new();
component.initialize()?;

// Always shutdown gracefully
component.shutdown()?;

// Check health periodically
if component.health_check() == HealthStatus::Unhealthy {
    component.restart()?;
}

// Implement all trait methods
impl Component for MyComponent {
    fn initialize(&mut self) -> Result<()> { /* ... */ }
    fn shutdown(&mut self) -> Result<()> { /* ... */ }
    fn health_check(&self) -> HealthStatus { /* ... */ }
}
```

---

## Migration Guide

### Migrating from Config to DatabaseConfig

**Old Code:**
```rust
use rusty_db::Config;

let config = Config::default();
let port = config.port;
let buffer_size = config.buffer_pool_size;
```

**New Code:**
```rust
use rusty_db::common::DatabaseConfig;

let config = DatabaseConfig::default();
let port = config.port;
let buffer_size = config.buffer_pool_size;

// Additional fields available
let isolation = config.default_isolation;
let max_conn = config.max_connections;
```

### Adding Custom Metrics

**Before:**
```rust
struct MyStats {
    counters: HashMap<String, u64>,
}
```

**After:**
```rust
use rusty_db::common::{ComponentStatistics, MetricValue};

let mut stats = ComponentStatistics::new("MyComponent".to_string());
stats.add_custom_metric("requests".to_string(), MetricValue::Counter(1000))?;
// Validates: <= MAX_CUSTOM_METRICS (1,000)
```

### Error Handling Updates

**Old Pattern:**
```rust
match operation() {
    Ok(result) => Ok(result),
    Err(e) => Err(format!("Operation failed: {}", e)),
}
```

**New Pattern:**
```rust
operation().map_err(|e| DbError::Internal(format!("Operation failed: {}", e)))?
```

### Snapshot Creation

**Old (Unchecked):**
```rust
let snapshot = Snapshot {
    snapshot_txn_id: 100,
    active_txns: vec![...], // Could be unbounded
    min_active_txn: 98,
    max_committed_txn: 97,
};
```

**New (Validated):**
```rust
let snapshot = Snapshot::new(100, vec![...], 98, 97)?;
// Validates: active_txns.len() <= MAX_ACTIVE_TRANSACTIONS (100,000)
```

---

## Appendix

### Version Information

**Library Version:** v0.5.1
**Build Information:**
```rust
use rusty_db::{VERSION, BUILD_INFO, print_info};

println!("{}", VERSION);      // "0.5.1"
println!("{}", BUILD_INFO);   // "RustyDB v0.5.1 - Enterprise Database Management System"
print_info();                 // Prints full version info
```

### Re-exports

The following types are re-exported from `lib.rs` for convenience:

```rust
pub use common::{
    Component, DatabaseConfig, HealthStatus, IndexId, IsolationLevel,
    Monitorable, PageId, Recoverable, ReplicableState, ResourceLimits,
    Schema, SystemEvent, TableId, TransactionId, Transactional,
    Tuple, Value,
};
pub use error::{DbError, Result};
```

**Usage:**
```rust
use rusty_db::{Result, DbError, Value, IsolationLevel};
// Instead of:
// use rusty_db::error::{Result, DbError};
// use rusty_db::common::{Value, IsolationLevel};
```

### File Locations

| Component | File Path | Lines |
|-----------|-----------|-------|
| Error types | `/home/user/rusty-db/src/error.rs` | 280 |
| Common types | `/home/user/rusty-db/src/common/mod.rs` | 1,242 |
| Library root | `/home/user/rusty-db/src/lib.rs` | 1,197 |

### Related Documentation

- **Architecture Overview:** `docs/ARCHITECTURE.md`
- **Development Guidelines:** `docs/DEVELOPMENT.md`
- **Security Architecture:** `docs/SECURITY_ARCHITECTURE.md`
- **Module Coordination:** `.scratchpad/COORDINATION_MASTER.md`

---

## Document Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-12-25 | Initial enterprise documentation for v0.5.1 |

---

**End of Document**

*RustyDB v0.5.1 - Enterprise Production Documentation*
*Copyright © 2025 RustyDB Project*
*Classification: Public*
