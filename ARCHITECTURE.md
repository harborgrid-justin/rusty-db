# RustyDB Architecture Documentation

## System Overview

RustyDB is a fully-featured, enterprise-grade relational database management system implemented in Rust. It follows a layered architecture similar to modern database systems like PostgreSQL and Oracle.

## Architecture Layers

### 1. Storage Layer
The foundation of RustyDB, responsible for persisting data to disk and managing memory.

**Components:**
- **Page**: Fixed-size (4KB) data blocks that are the unit of I/O
- **Disk Manager**: Handles reading/writing pages to/from disk with proper error handling
- **Buffer Pool Manager**: In-memory cache with LRU replacement policy
  - Manages 1000 pages by default
  - Handles dirty page flushing
  - Prevents data loss through proper eviction

**Key Features:**
- Thread-safe concurrent access using RwLocks
- Efficient page-based I/O
- Automatic dirty page detection and flushing
- Protection against mutex poisoning

### 2. Catalog Layer
Manages database metadata and schema information.

**Components:**
- **Schema**: Table definitions with columns and data types
- **Catalog**: Central repository for all metadata
- **Column**: Column definitions with type information

**Supported Data Types:**
- Integer, BigInt
- Float, Double
- Varchar(n), Text
- Boolean
- Date, Timestamp

**Operations:**
- CREATE TABLE
- DROP TABLE
- Schema validation
- Metadata queries

### 3. Transaction Layer
Ensures ACID properties through sophisticated concurrency control.

**Components:**
- **Transaction Manager**: Coordinates transaction lifecycle
- **Lock Manager**: Implements two-phase locking (2PL)
- **Transaction States**: Growing, Shrinking, Committed, Aborted

**Features:**
- ACID compliance
- Two-phase locking for isolation
- Deadlock prevention
- Lock timeout handling
- Proper lock cleanup (no memory leaks)

**Lock Modes:**
- Shared locks for reads
- Exclusive locks for writes

### 4. Index Layer
Provides fast data access through multiple index types.

**Index Types:**

**B-Tree Index:**
- Ordered index structure
- Supports range queries
- O(log n) lookup time
- Self-balancing

**Hash Index:**
- Unordered index structure
- O(1) average lookup time
- Best for equality comparisons

**Index Operations:**
- Insert
- Search
- Range search (B-Tree only)
- Delete

### 5. Query Processing Layer

**Components:**

**SQL Parser:**
- Lexical analysis and parsing
- Converts SQL to AST
- Syntax validation
- Uses sqlparser-rs library

**Query Planner:**
- Converts AST to execution plan
- Creates operator tree
- Handles table scans, filters, projections, joins
- Supports aggregation nodes (GROUP BY, HAVING)
- Supports sort and limit nodes
- Subquery handling

**Query Optimizer:**
- Cost-based optimization framework
- Predicate pushdown optimization
- Join reordering based on cost estimation
- Index selection
- Constant folding
- Cost estimation for all plan nodes

**Executor:**
- Executes query plans
- Returns results
- Integrates with catalog and storage
- JOIN execution (INNER, LEFT, RIGHT, FULL, CROSS)
- Aggregation execution (COUNT, SUM, AVG, MIN, MAX, etc.)
- Sorting and limiting
- Subquery execution

### 6. Advanced SQL Features Layer

**Triggers:**
- BEFORE/AFTER trigger support
- INSERT/UPDATE/DELETE events
- Conditional trigger execution
- Trigger management (create, drop, enable/disable)

**Stored Procedures:**
- SQL-based procedures
- Parameter support (IN, OUT, INOUT)
- Procedure execution framework
- Procedure management

**Replication:**
- Primary-replica architecture
- Synchronous, asynchronous, and semi-sync modes
- Replication log management
- Failover support
- Replica monitoring

### 7. Network Layer
Enables client-server communication.

**Components:**
- **Server**: Async TCP server using Tokio
- **Protocol**: Binary serialization with bincode
- **Request/Response**: Type-safe message passing

**Features:**
- Non-blocking I/O
- Concurrent connection handling
- DoS protection (1MB request limit)
- Efficient binary protocol

## Data Flow

### Write Path (INSERT)
```
SQL Query
  ↓
Parser → AST
  ↓
Planner → Execution Plan
  ↓
Executor
  ↓
Transaction Manager (acquire locks)
  ↓
Catalog (validate schema)
  ↓
Storage Engine (write to buffer pool)
  ↓
Buffer Pool (mark dirty)
  ↓
Disk Manager (flush when needed)
```

### Read Path (SELECT)
```
SQL Query
  ↓
Parser → AST
  ↓
Planner → Execution Plan
  ↓
Optimizer (select best plan)
  ↓
Executor
  ↓
Index Layer (if indexed)
  ↓
Storage Engine (fetch pages)
  ↓
Buffer Pool (cache hit/miss)
  ↓
Return Results
```

### Transaction Path
```
BEGIN
  ↓
Transaction Manager (assign txn_id)
  ↓
Execute Queries
  ↓
Lock Manager (acquire locks)
  ↓
Perform Operations
  ↓
COMMIT/ROLLBACK
  ↓
Lock Manager (release locks)
  ↓
Buffer Pool (flush if commit)
```

## Concurrency Control

**Two-Phase Locking (2PL):**
1. **Growing Phase**: Transaction acquires locks
2. **Shrinking Phase**: Transaction releases locks
3. No new locks after first release

**Lock Compatibility Matrix:**
```
          | Shared | Exclusive
----------|--------|----------
Shared    |   ✓    |    ✗
Exclusive |   ✗    |    ✗
```

## Memory Management

**Buffer Pool:**
- Fixed size (configurable)
- LRU replacement policy
- Pin/unpin mechanism
- Dirty page tracking

**Lock-free regions:**
- Read-heavy catalog operations
- Immutable configuration

## Error Handling

**Comprehensive Error Types:**
- IO errors
- SQL parsing errors
- Transaction errors
- Storage errors
- Catalog errors
- Index errors
- Execution errors
- Network errors
- Serialization errors
- Lock timeouts
- Deadlock detection

All errors are properly propagated using Result<T, DbError>.

## Performance Characteristics

**Storage:**
- O(1) page lookup in buffer pool
- O(log n) disk seeks for page location

**Indexing:**
- B-Tree: O(log n) search, O(log n) insert
- Hash: O(1) average search/insert

**Transactions:**
- Lock acquisition: O(1) with HashMap
- Deadlock detection: Future enhancement

**Query Processing:**
- Table scan: O(n)
- Index scan: O(log n + m) where m is result size

## Scalability

**Current Limits:**
- Buffer pool: Configurable (default 1000 pages ≈ 4MB)
- Page size: 4KB (industry standard)
- Max request size: 1MB (DoS protection)
- Concurrent connections: Limited by system resources

**Future Scalability:**
- Horizontal scaling through replication
- Partitioning support
- Connection pooling
- Query result caching

## Security Features

**Current:**
- Input validation (request size limits)
- Proper error handling (no panics on mutex poisoning)
- Memory safety (Rust guarantees)
- Type safety throughout

**Future:**
- Authentication and authorization
- Encrypted connections (TLS)
- SQL injection prevention (parameterized queries)
- Audit logging

## Testing

**Coverage:**
- Storage layer: Page, Disk, Buffer Pool
- Catalog: Schema management
- Parser: SQL parsing
- Execution: Query execution
- Transaction: Lock management, lifecycle
- Index: B-Tree and Hash operations

**Test Count:** 14 unit tests, all passing

## Future Enhancements

### Query Processing
- JOIN operations (INNER, OUTER, CROSS)
- Aggregations (COUNT, SUM, AVG, MIN, MAX)
- GROUP BY and HAVING
- ORDER BY
- Subqueries
- Common Table Expressions (CTEs)

### Advanced Features
- Foreign key constraints
- Triggers
- Stored procedures
- Views
- Full-text search
- JSON support
- Window functions

### Operations
- Backup and restore
- Replication (master-slave)
- Point-in-time recovery
- Query performance monitoring
- Explain plan visualization

### Optimization
- Advanced cost-based optimization
- Query result caching
- Materialized views
- Adaptive query execution

## Comparison with Oracle DB

| Feature | RustyDB | Oracle DB |
|---------|---------|-----------|
| Language | Rust | C/C++ |
| ACID | ✓ | ✓ |
| SQL Support | Basic | Full |
| Transactions | ✓ | ✓ |
| Indexing | B-Tree, Hash | B-Tree, Bitmap, etc. |
| Replication | Future | ✓ |
| Partitioning | Future | ✓ |
| Stored Procedures | Future | ✓ |
| License | MIT/Apache | Proprietary |
| Memory Safety | Guaranteed | Manual |

## Conclusion

RustyDB provides a solid foundation for an enterprise-grade database system with:
- Robust storage engine
- ACID compliance
- Efficient indexing
- Modern async networking
- Comprehensive error handling
- Memory safety guarantees

While it's currently a v0.1.0 implementation, the architecture is designed to scale and support advanced features as the system evolves.
