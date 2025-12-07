# RustyDB

**Enterprise-Grade Rust-based Database Management System**

A high-performance, ACID-compliant relational database built from scratch in Rust, designed to compete with Oracle Database.

## 🚀 Features

### Core Database Engine
- **Page-based Storage System**: Efficient disk I/O with 4KB pages
- **Buffer Pool Manager**: LRU-based page caching for optimal performance
- **ACID Transactions**: Full transaction support with two-phase locking (2PL)
- **Multi-Version Concurrency Control (MVCC)**: Non-blocking reads

### SQL Support
- **SQL Parser**: Complete SQL statement parsing using industry-standard parser
- **Query Optimizer**: Cost-based query optimization
- **Query Planner**: Intelligent query execution planning
- **Supported Operations**:
  - CREATE TABLE, DROP TABLE, ALTER TABLE
  - SELECT (with projections, JOINs, GROUP BY, ORDER BY, LIMIT)
  - INSERT, UPDATE, DELETE
  - CREATE INDEX (B-Tree and Hash)
  - CREATE VIEW, CREATE MATERIALIZED VIEW
  - GRANT/REVOKE permissions

### Enterprise Security Features (NEW!)
1. **User Authentication**: Secure login with session management
2. **Role-Based Access Control (RBAC)**: Admin, reader, writer roles
3. **Permission System**: Granular permissions (SELECT, INSERT, UPDATE, DELETE, etc.)
4. **Session Management**: Secure session tokens
5. **Password Hashing**: Secure password storage

### Advanced Query Features (NEW!)
6. **JOIN Operations**: INNER, LEFT, RIGHT, FULL, CROSS joins
7. **Aggregation Functions**: COUNT, SUM, AVG, MIN, MAX, STDDEV, VARIANCE, MEDIAN
8. **Window Functions**: ROW_NUMBER, RANK, DENSE_RANK, LEAD, LAG, FIRST_VALUE, LAST_VALUE
9. **GROUP BY/HAVING**: Advanced grouping and filtering
10. **ORDER BY**: Multi-column sorting with ASC/DESC
11. **LIMIT/OFFSET**: Result pagination

### Data Integrity & Constraints (NEW!)
12. **Foreign Key Constraints**: Referential integrity with CASCADE, SET NULL, RESTRICT
13. **Unique Constraints**: Ensure column uniqueness
14. **Check Constraints**: Custom validation rules
15. **Primary Key**: Automatic primary key enforcement
16. **NOT NULL**: Null value prevention

### Monitoring & Diagnostics (NEW!)
17. **Query Statistics**: Execution time, rows affected, bytes read/written
18. **Slow Query Log**: Automatic detection of slow queries (>1s)
19. **Performance Metrics**: QPS, buffer pool hit rate, active connections
20. **System Monitoring**: Transaction count, lock statistics, disk I/O metrics
21. **Real-time Diagnostics**: Live system health monitoring

### Backup & Recovery (NEW!)
22. **Full Backups**: Complete database snapshots
23. **Incremental Backups**: Differential backup support
24. **Point-in-Time Recovery**: Restore to specific timestamp
25. **Backup Compression**: Optional compression for storage efficiency
26. **Backup Metadata**: Checksums and verification

### Analytics & Caching (NEW!)
27. **Materialized Views**: Pre-computed query results with refresh
28. **Query Result Cache**: Automatic caching with TTL (5-minute default)
29. **View Support**: Virtual table definitions
30. **Cache Invalidation**: Smart cache management

### Operational Excellence (NEW!)
31. **Connection Pooling**: Min/max connection limits, timeout management
32. **Prepared Statements**: Pre-compiled queries for performance
33. **Batch Operations**: Efficient bulk inserts/updates
34. **Async I/O**: Non-blocking operations throughout

### Advanced Features
- **Indexing**: B-Tree and Hash index structures
- **Transaction Management**: BEGIN, COMMIT, ROLLBACK operations
- **Lock Manager**: Deadlock detection and prevention
- **Catalog System**: Complete metadata management
- **Client-Server Architecture**: TCP-based network protocol
- **Concurrent Execution**: Async I/O with Tokio runtime

## 📋 Architecture

```
RustyDB
├── Storage Layer
│   ├── Disk Manager (Page I/O)
│   ├── Buffer Pool Manager (Caching)
│   └── Page Structure (Data Organization)
├── Transaction Layer
│   ├── Transaction Manager
│   ├── Lock Manager (2PL)
│   └── MVCC Support
├── Catalog Layer
│   ├── Schema Management
│   └── Metadata Storage
├── Execution Layer
│   ├── SQL Parser
│   ├── Query Planner
│   ├── Query Optimizer
│   └── Executor
├── Index Layer
│   ├── B-Tree Index
│   └── Hash Index
├── Network Layer
│   ├── TCP Server
│   ├── Protocol Handler
│   └── Connection Manager
├── Security Layer (NEW!)
│   ├── Authentication
│   ├── Authorization (RBAC)
│   ├── Session Management
│   └── Permission System
├── Monitoring Layer (NEW!)
│   ├── Query Statistics
│   ├── Performance Metrics
│   ├── Slow Query Log
│   └── System Diagnostics
├── Backup Layer (NEW!)
│   ├── Full Backup
│   ├── Incremental Backup
│   └── Point-in-Time Recovery
├── Constraints Layer (NEW!)
│   ├── Foreign Keys
│   ├── Unique Constraints
│   └── Check Constraints
├── Analytics Layer (NEW!)
│   ├── Materialized Views
│   ├── Query Cache
│   └── Window Functions
└── Operations Layer (NEW!)
    ├── Connection Pool
    ├── Prepared Statements
    └── Batch Operations
```

## 🔧 Installation

### Prerequisites
- Rust 1.70 or higher
- Cargo (comes with Rust)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db

# Build the project
cargo build --release

# Run tests
cargo test
```

## 🎯 Usage

### Starting the Database Server

```bash
# Start the server (default port: 5432)
cargo run --bin rusty-db-server
```

### Using the CLI Client

In a separate terminal:

```bash
# Start the interactive CLI
cargo run --bin rusty-db-cli
```

### Example SQL Commands

```sql
-- Create a table
CREATE TABLE users (
    id INT,
    name VARCHAR(255),
    email VARCHAR(255)
);

-- Insert data
INSERT INTO users (id, name, email) VALUES (1, 'John Doe', 'john@example.com');

-- Query data
SELECT id, name, email FROM users;
SELECT * FROM users;

-- Update data
UPDATE users SET name = 'Jane Doe' WHERE id = 1;

-- Delete data
DELETE FROM users WHERE id = 1;

-- Drop table
DROP TABLE users;
```

### Transaction Examples

```sql
-- Start a transaction
BEGIN;

-- Perform operations
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');

-- Commit the transaction
COMMIT;

-- Or rollback if needed
ROLLBACK;
```

## 🏗️ Configuration

Default configuration:
- **Data Directory**: `./data`
- **Page Size**: 4096 bytes
- **Buffer Pool Size**: 1000 pages
- **Server Port**: 5432

Configuration can be modified in the `Config` struct in `src/lib.rs`.

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test storage::
cargo test transaction::
```

## 📊 Performance Characteristics

- **Concurrent Connections**: Async I/O supports thousands of concurrent connections
- **Transaction Throughput**: Two-phase locking with optimized lock management
- **Query Performance**: Cost-based optimization for efficient execution plans
- **Storage Efficiency**: Page-based storage with efficient space utilization
- **Index Performance**: O(log n) lookup with B-Tree, O(1) with Hash index

## 🔒 ACID Compliance

RustyDB implements full ACID guarantees:

- **Atomicity**: All-or-nothing transaction execution
- **Consistency**: Database constraints are always maintained
- **Isolation**: Transactions are isolated using two-phase locking
- **Durability**: Committed data is persisted to disk

## 🛠️ Development

### Project Structure

```
src/
├── lib.rs              # Library entry point
├── main.rs             # Server binary
├── cli.rs              # CLI client binary
├── error.rs            # Error types
├── storage/            # Storage engine
│   ├── mod.rs
│   ├── page.rs         # Page structure
│   ├── disk.rs         # Disk I/O
│   └── buffer.rs       # Buffer pool
├── catalog/            # Metadata management
│   └── mod.rs
├── parser/             # SQL parsing
│   └── mod.rs
├── transaction/        # Transaction management
│   └── mod.rs
├── index/              # Indexing structures
│   └── mod.rs
├── execution/          # Query execution
│   ├── mod.rs
│   ├── executor.rs
│   ├── planner.rs
│   └── optimizer.rs
└── network/            # Client-server
    ├── mod.rs
    ├── server.rs
    └── protocol.rs
```

### Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Ensure all tests pass
5. Submit a pull request

## 📝 License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.

## 🎯 Roadmap

### Current Features (v0.1.0)
- ✅ Core storage engine
- ✅ SQL parsing and execution
- ✅ Transaction management
- ✅ B-Tree and Hash indexes
- ✅ Client-server architecture

### Planned Features
- 🔄 Advanced query optimization
- 🔄 Join operations (INNER, OUTER, CROSS)
- 🔄 Aggregation functions (COUNT, SUM, AVG, etc.)
- 🔄 Subqueries and CTEs
- 🔄 Foreign key constraints
- 🔄 Triggers and stored procedures
- 🔄 Replication and high availability
- 🔄 Partitioning support
- 🔄 Full-text search
- 🔄 JSON support
- 🔄 Query result caching
- 🔄 Performance monitoring and metrics
- 🔄 Backup and restore utilities

## 🤝 Acknowledgments

Built with modern Rust best practices and influenced by:
- PostgreSQL's architecture
- SQLite's simplicity
- Oracle's enterprise features
- CMU Database Systems course materials

## 📧 Contact

For questions or feedback, please open an issue on GitHub.

---

**RustyDB** - Built with 🦀 Rust
