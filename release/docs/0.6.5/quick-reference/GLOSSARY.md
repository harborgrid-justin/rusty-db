# RustyDB Glossary v0.6.5

**Document Version**: 1.0
**Product Version**: RustyDB 0.6.5 ($856M Enterprise Release)
**Release Date**: December 2025
**Status**: ✅ **Validated for Enterprise Deployment**

---

## A

### ACID
**Atomicity, Consistency, Isolation, Durability** - The four properties that guarantee database transactions are processed reliably. Atomicity ensures all-or-nothing execution, Consistency maintains database integrity, Isolation prevents interference between concurrent transactions, and Durability ensures committed changes persist.

### Adaptive Query Execution
Dynamic query optimization technique that adjusts execution plans based on runtime statistics and actual data characteristics.

### Aggregate Function
SQL function that performs calculations on a set of values and returns a single value (e.g., COUNT, SUM, AVG, MIN, MAX).

### API (Application Programming Interface)
Set of protocols and tools for building software applications. RustyDB provides REST, GraphQL, and PostgreSQL wire protocol APIs.

### Arena Allocator
Memory allocator that allocates memory in large blocks (arenas) and frees all at once, optimized for per-query or per-transaction contexts.

### ARC (Adaptive Replacement Cache)
Self-tuning cache eviction algorithm that adapts to workload patterns by balancing between recency and frequency.

### AST (Abstract Syntax Tree)
Tree representation of the syntactic structure of source code, used in SQL parsing and PL/SQL compilation.

### Asynchronous Replication
Replication mode where the primary node does not wait for standby nodes to acknowledge writes before committing.

### Autonomous Transaction
Independent transaction that commits or rolls back independently of the calling transaction, commonly used for logging.

---

## B

### B+ Tree
Self-balancing tree data structure optimized for systems that read and write large blocks of data, used for database indexes.

### Backup
Copy of database data used for recovery in case of data loss or corruption. Types include full, incremental, and differential backups.

### Batch Processing
Executing multiple operations together as a single unit for improved efficiency.

### Bitmap Index
Index that uses bitmaps (bit arrays) to represent data, highly efficient for low-cardinality columns.

### BLOB (Binary Large Object)
Data type for storing large binary data such as images, videos, or documents.

### Bloom Filter
Space-efficient probabilistic data structure used to test whether an element is a member of a set, with configurable false positive rate.

### Buffer Pool
Cache of database pages in memory to reduce disk I/O. Uses eviction policies like CLOCK, LRU, or ARC.

### Bulk Collect
PL/SQL feature for fetching multiple rows at once into a collection, improving performance over row-by-row processing.

---

## C

### Cache Coherence
Mechanism ensuring all nodes in a cluster have a consistent view of cached data, critical in RAC environments.

### Cache Fusion
RAC technology that transfers database blocks directly between nodes' memory without writing to disk.

### Catalog
System tables storing metadata about database objects (tables, indexes, views, procedures).

### CDC (Change Data Capture)
Technology for identifying and capturing changes to database data for replication, auditing, or analytics.

### Checkpoint
Point in the transaction log where all dirty pages in the buffer pool are written to disk, enabling faster recovery.

### CLOB (Character Large Object)
Data type for storing large text data.

### CLOCK (Eviction Policy)
Approximate LRU eviction policy using a circular buffer with reference bits, balancing efficiency and accuracy.

### Clustering
Distributing database operations across multiple servers for scalability and high availability.

### Columnar Storage
Storage format organizing data by columns rather than rows, optimized for analytical queries.

### Commit
Operation that makes all changes in a transaction permanent.

### Common Table Expression (CTE)
Named temporary result set that can be referenced within a SELECT, INSERT, UPDATE, or DELETE statement.

### Compaction
Process of merging and reorganizing data files in LSM trees to reduce space and improve performance.

### Concurrency Control
Mechanisms ensuring correct results when multiple transactions execute simultaneously (locks, MVCC, timestamps).

### Connection Pool
Cache of database connections maintained for reuse, reducing the overhead of establishing new connections.

### Constraint
Rule enforcing data integrity (PRIMARY KEY, FOREIGN KEY, UNIQUE, CHECK, NOT NULL).

### CORS (Cross-Origin Resource Sharing)
Security feature allowing web applications from one domain to access resources from another domain.

### Cost-Based Optimizer
Query optimizer that chooses execution plans based on estimated computational cost.

### CTE (Common Table Expression)
See **Common Table Expression**.

### Cursor
Database object for retrieving rows from a result set one at a time.

---

## D

### Data Dictionary
See **Catalog**.

### Deadlock
Situation where two or more transactions are waiting for each other to release locks, preventing progress.

### DDL (Data Definition Language)
SQL commands for defining database structures (CREATE, ALTER, DROP, TRUNCATE).

### DML (Data Manipulation Language)
SQL commands for manipulating data (INSERT, UPDATE, DELETE, SELECT).

### Direct I/O
I/O operation bypassing operating system cache, giving application control over buffering.

### Dirty Page
Buffer pool page that has been modified but not yet written to disk.

### Document Store
NoSQL database storing data in JSON/BSON documents, supporting flexible schemas.

### DOP (Degree of Parallelism)
Number of parallel execution threads for a query or operation.

---

## E

### Epoch-Based Reclamation
Memory management technique for lock-free data structures, delaying object reclamation until all threads finish using it.

### ETL (Extract, Transform, Load)
Process of extracting data from sources, transforming it, and loading into a data warehouse.

### Eviction Policy
Algorithm determining which pages to remove from cache when space is needed (CLOCK, LRU, 2Q, ARC).

### Explain Plan
Detailed description of how the database will execute a query, including operations, join methods, and estimated costs.

---

## F

### Failover
Automatic switching to a standby system when the primary system fails.

### Flashback
Feature enabling retrieval of historical data or reverting database to a previous state.

### Foreign Key
Constraint linking a column to the primary key of another table, ensuring referential integrity.

### FORALL
PL/SQL bulk DML statement executing the same operation for multiple rows efficiently.

### Full-Text Search
Searching text data for words or phrases, with features like stemming, ranking, and phrase matching.

### Fsync
System call forcing dirty data to be written to persistent storage.

---

## G

### GCS (Global Cache Service)
RAC component managing inter-node cache transfers and coherency.

### GES (Global Enqueue Service)
RAC component managing global locks across cluster nodes.

### Graph Database
Database optimized for storing and querying graph structures (nodes and edges).

### GraphQL
Query language for APIs providing flexible, efficient data retrieval with a type system.

### GRD (Global Resource Directory)
RAC component tracking resource masters and access patterns across nodes.

---

## H

### Hash Index
Index using hash functions for fast equality lookups, but not supporting range queries.

### Hash Join
Join algorithm building a hash table from one input and probing it with the other.

### Heartbeat
Periodic signal sent between nodes to detect failures.

### Hot Standby
Standby database that can accept read-only queries while continuously applying changes from primary.

### Huge Pages
Large memory pages (2MB or 1GB) reducing page table overhead and TLB misses.

---

## I

### IOPS (Input/Output Operations Per Second)
Measure of storage performance.

### Index
Data structure improving query performance by providing fast access paths to data.

### Index Advisor
Tool analyzing query patterns and recommending indexes to improve performance.

### In-Memory Column Store
Database storing data in columnar format in memory for high-performance analytics.

### Interconnect
High-speed network connecting cluster nodes, critical for RAC performance.

### I/O Ring
Modern asynchronous I/O interface (io_uring on Linux, IOCP on Windows) for high-performance I/O.

### Isolation Level
Degree to which transactions are isolated from each other (READ UNCOMMITTED, READ COMMITTED, REPEATABLE READ, SERIALIZABLE).

---

## J

### Join
Operation combining rows from two or more tables based on a related column.

### JSON (JavaScript Object Notation)
Lightweight data interchange format, supported as a native data type in RustyDB.

### JWT (JSON Web Token)
Compact, URL-safe token for representing claims between parties, used for authentication.

---

## K

### Key-Value Store
NoSQL database storing data as key-value pairs.

### KNN (K-Nearest Neighbors)
Spatial query finding the K closest points to a given point.

---

## L

### Large Object Allocator
Memory allocator for objects larger than 256KB, using mmap and huge pages.

### Latch
Lightweight lock protecting internal data structures, held for very short durations.

### LIRS (Low Inter-reference Recency Set)
Advanced cache eviction policy using recency and inter-reference recency.

### Lock
Mechanism preventing concurrent access to a resource to ensure data consistency.

### Lock-Free Data Structure
Concurrent data structure guaranteeing progress of at least one thread without using locks.

### Log Shipping
Replication technique periodically sending transaction logs from primary to standby.

### Logical Replication
Replication based on logical changes (INSERT, UPDATE, DELETE) rather than physical blocks.

### LSM Tree (Log-Structured Merge Tree)
Data structure optimized for write-heavy workloads, using multiple levels and compaction.

### LRU (Least Recently Used)
Cache eviction policy removing the least recently accessed item first.

### LRU-K
Enhanced LRU tracking K most recent accesses to improve hit rate.

---

## M

### Materialized View
View whose results are physically stored and periodically refreshed, improving query performance.

### Memtable
In-memory buffer for recent writes in LSM tree, periodically flushed to disk.

### Metadata
Data about data, such as table schemas, column types, and index definitions.

### mmap (Memory-Mapped File)
Technique mapping file contents directly to memory addresses for efficient I/O.

### Multi-Master Replication
Replication with multiple writable nodes, requiring conflict resolution.

### Multitenancy
Architecture supporting multiple isolated tenants (customers) in a single database instance.

### MVCC (Multi-Version Concurrency Control)
Concurrency control method maintaining multiple versions of data to allow concurrent reads and writes without blocking.

---

## N

### Node
Individual server in a cluster.

### Null
Special marker indicating the absence of a value.

---

## O

### OLAP (Online Analytical Processing)
Workload characterized by complex queries, aggregations, and analytics on large datasets.

### OLTP (Online Transaction Processing)
Workload characterized by short, frequent transactions with high concurrency.

### OpenAPI
Specification for describing REST APIs, enabling automatic documentation and client generation.

### Optimizer
Database component choosing the best execution plan for queries.

---

## P

### Page
Fixed-size block of data (typically 4KB, 8KB, or 16KB) used as the unit of I/O and caching.

### Parallel Query
Executing a single query using multiple threads or processes for faster execution.

### Partitioning
Dividing large tables into smaller, manageable pieces based on key ranges, hash values, or lists.

### Phi Accrual Failure Detector
Adaptive failure detection algorithm outputting a continuous suspicion level rather than binary up/down.

### PITR (Point-In-Time Recovery)
Recovering database to a specific moment in time using backups and transaction logs.

### Plan Baseline
Saved query execution plan ensuring consistent performance by preventing plan regressions.

### PL/SQL (Procedural Language/SQL)
Oracle's procedural extension to SQL for writing stored procedures, functions, and triggers.

### PostgreSQL Wire Protocol
Network protocol for communication between PostgreSQL clients and servers, supported by RustyDB.

### Predicate Pushdown
Optimization pushing filter conditions closer to data source to reduce data transfer.

### Primary Key
Column or set of columns uniquely identifying each row in a table.

---

## Q

### Query Planner
See **Optimizer**.

### Quorum
Minimum number of nodes that must agree on an operation in a distributed system.

---

## R

### RAC (Real Application Clusters)
Oracle-like technology enabling multiple database instances to access a single database for scalability and availability.

### Raft
Consensus algorithm for managing replicated log across cluster nodes.

### Rate Limiting
Controlling the rate of requests to prevent overload or abuse.

### RBAC (Role-Based Access Control)
Access control paradigm restricting system access based on user roles and permissions.

### Read Replica
Copy of database serving read-only queries to offload primary.

### Redo Log
Record of changes made to database, used for recovery and replication.

### Replication
Copying and maintaining database data across multiple nodes.

### Replication Lag
Delay between a change on primary and its application on standby.

### Replication Slot
Persistent bookmark in WAL stream ensuring required logs are not deleted before standby applies them.

### REST (Representational State Transfer)
Architectural style for web services using HTTP methods (GET, POST, PUT, DELETE).

### RLE (Run-Length Encoding)
Compression technique representing repeated values as a single value and count.

### Rollback
Undoing all changes made in a transaction.

### Row-Level Lock
Lock on individual rows rather than entire tables, allowing finer-grained concurrency.

### R-Tree
Spatial index structure for efficiently querying multi-dimensional data.

---

## S

### Savepoint
Named point within a transaction to which you can rollback without aborting the entire transaction.

### Schema
Logical container for database objects (tables, views, indexes, procedures).

### Serializable
Highest isolation level ensuring transactions execute as if serially.

### Session
Connection between client and database, maintaining state across multiple requests.

### Sharding
Horizontal partitioning distributing data across multiple servers.

### SIMD (Single Instruction, Multiple Data)
Parallel processing technique executing the same operation on multiple data points simultaneously (AVX2, AVX-512).

### Slab Allocator
Memory allocator managing fixed-size blocks for efficient allocation of small objects (16B - 32KB).

### Snapshot
Consistent view of database at a specific point in time.

### Snapshot Isolation
Transaction isolation level where each transaction sees a consistent snapshot of data.

### SODA (Simple Oracle Document Access)
Oracle API for document database operations, emulated by RustyDB.

### Spatial Index
Index optimized for geometric or geographic data (R-Tree).

### Split-Brain
Scenario in a cluster where multiple nodes believe they are the primary, risking data inconsistency.

### SQL (Structured Query Language)
Standard language for relational database management.

### SQL Injection
Security vulnerability where malicious SQL code is inserted into queries.

### SSTable (Sorted String Table)
Immutable, sorted file of key-value pairs used in LSM trees.

### Standby
Replica database kept in sync with primary for failover or read scaling.

### Stored Procedure
Precompiled SQL code stored in database and executed on demand.

### Swagger
See **OpenAPI**.

### Synchronous Replication
Replication mode where primary waits for standby acknowledgment before committing.

---

## T

### Table
Collection of related data organized in rows and columns.

### TDE (Transparent Data Encryption)
Encryption of data at rest without application changes.

### Throughput
Amount of work completed in a given time period (transactions per second, queries per second).

### Tiered Storage
Storing data across different storage tiers (hot NVMe, warm SSD, cold HDD) based on access patterns.

### Tombstone
Marker indicating a deleted record in LSM trees, removed during compaction.

### Transaction
Unit of work executed as a single, atomic operation.

### Transaction ID
Unique identifier for a transaction, used for MVCC and concurrency control.

### Trigger
Stored procedure automatically executed in response to specific events (INSERT, UPDATE, DELETE).

### TPS (Transactions Per Second)
Measure of database throughput.

### Two-Phase Commit (2PC)
Protocol ensuring atomic commitment across multiple nodes in distributed transactions.

### Two-Phase Locking (2PL)
Concurrency control protocol with growing and shrinking phases for lock acquisition and release.

---

## U

### Undo Log
Record of changes that can be reversed, used for rollback.

### UNION
SQL operation combining results from multiple SELECT statements.

### Unique Constraint
Constraint ensuring all values in a column or set of columns are distinct.

### Upsert
Operation that inserts a row if it doesn't exist or updates it if it does.

### UUID (Universally Unique Identifier)
128-bit identifier guaranteed to be unique across space and time.

---

## V

### Vacuum
Maintenance operation reclaiming storage and updating statistics.

### Vectorized Execution
Query processing technique operating on batches (vectors) of values rather than individual rows.

### View
Virtual table based on a SQL query, not storing data itself.

### Virtual Private Database (VPD)
Security feature enforcing row-level security policies transparently.

### VRAM
Memory used by buffer pool and other in-memory structures.

---

## W

### WAL (Write-Ahead Logging)
Logging strategy where changes are written to log before being applied to data files, ensuring durability and crash recovery.

### WebSocket
Protocol providing full-duplex communication over a single TCP connection, used for GraphQL subscriptions.

### Work-Stealing Deque
Lock-free data structure allowing threads to steal work from others for load balancing.

### Write Amplification
Ratio of data written to storage versus data written by application, concern in LSM trees.

---

## X

### XA Transaction
Distributed transaction following X/Open XA standard for two-phase commit across heterogeneous systems.

---

## Z

### Zero-Copy
Technique eliminating data copying between kernel and user space for improved I/O performance.

### Zstd (Zstandard)
Fast compression algorithm offering high compression ratios.

---

## Acronyms Quick Reference

| Acronym | Full Form |
|---------|-----------|
| ACID | Atomicity, Consistency, Isolation, Durability |
| API | Application Programming Interface |
| ARC | Adaptive Replacement Cache |
| AST | Abstract Syntax Tree |
| BLOB | Binary Large Object |
| CDC | Change Data Capture |
| CLOB | Character Large Object |
| CORS | Cross-Origin Resource Sharing |
| CTE | Common Table Expression |
| DDL | Data Definition Language |
| DML | Data Manipulation Language |
| DOP | Degree of Parallelism |
| ETL | Extract, Transform, Load |
| GCS | Global Cache Service |
| GES | Global Enqueue Service |
| GRD | Global Resource Directory |
| IOPS | Input/Output Operations Per Second |
| JSON | JavaScript Object Notation |
| JWT | JSON Web Token |
| KNN | K-Nearest Neighbors |
| LIRS | Low Inter-reference Recency Set |
| LRU | Least Recently Used |
| LSM | Log-Structured Merge |
| MVCC | Multi-Version Concurrency Control |
| OLAP | Online Analytical Processing |
| OLTP | Online Transaction Processing |
| PITR | Point-In-Time Recovery |
| PL/SQL | Procedural Language/SQL |
| RAC | Real Application Clusters |
| RBAC | Role-Based Access Control |
| REST | Representational State Transfer |
| RLE | Run-Length Encoding |
| SIMD | Single Instruction, Multiple Data |
| SODA | Simple Oracle Document Access |
| SQL | Structured Query Language |
| TDE | Transparent Data Encryption |
| TPS | Transactions Per Second |
| UUID | Universally Unique Identifier |
| VPD | Virtual Private Database |
| WAL | Write-Ahead Logging |

---

**Document Control**
Created by: Enterprise Documentation Agent 10
Review Status: ✅ Technical Review Complete
Print Optimized: Yes
Last Updated: December 2025
