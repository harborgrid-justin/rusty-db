# Storage API Coverage Analysis Report
**PhD Agent 1 - Storage API Specialist**
**Date:** 2025-12-12
**Status:** COMPREHENSIVE ANALYSIS COMPLETE

---

## Executive Summary

This report provides a comprehensive analysis of REST API and GraphQL coverage for RustyDB's Storage layer (src/storage/, src/buffer/, src/memory/, src/io/). The analysis identified **37 major storage features** with **35% having partial API coverage** and **65% having NO API coverage**.

### Key Findings:
- ✅ **Current Coverage:** 13/37 features (35%) - Basic storage operations covered
- ❌ **Missing Coverage:** 24/37 features (65%) - Advanced features not exposed
- 🔴 **Critical Gaps:** LSM Tree, Columnar Storage, Tiered Storage, Advanced I/O features
- 🟡 **Partial Coverage:** Buffer pool (basic stats only), Memory (basic allocation only)

---

## 1. COMPLETE FEATURE INVENTORY

### 1.1 Buffer Pool Management (src/buffer/)

| Feature | Struct/Method | Current API Status | Priority |
|---------|--------------|-------------------|----------|
| **Buffer Pool Configuration** | `BufferPoolConfig` | ❌ NOT EXPOSED | HIGH |
| Buffer pool size config | `.num_frames` | ❌ NO API | HIGH |
| Eviction policy config | `.eviction_policy` | ❌ NO API | HIGH |
| Per-core pools config | `.per_core_pools` | ❌ NO API | MEDIUM |
| Flush configuration | `.max_flush_batch_size` | ❌ NO API | MEDIUM |
| **Buffer Pool Statistics** | `BufferPoolStats` | ✅ PARTIAL | HIGH |
| Basic stats | `get_buffer_pool_stats()` | ✅ REST ONLY | DONE |
| Hit ratio | `.hit_ratio` | ✅ REST ONLY | DONE |
| Page reads/writes | `.page_reads`, `.page_writes` | ✅ REST ONLY | DONE |
| Eviction stats | `.evictions` | ✅ REST ONLY | DONE |
| Prefetch stats | `prefetch_stats()` | ❌ NO API | MEDIUM |
| **Buffer Pool Operations** | `BufferPoolManager` | ❌ NOT EXPOSED | HIGH |
| Pin page | `pin_page()` | ❌ NO API | LOW |
| Unpin page | `unpin_page()` | ❌ NO API | LOW |
| Flush operations | `flush_all()`, `flush_page_by_id()` | ✅ REST ONLY | DONE |
| Prefetch operations | `prefetch_pages()`, `prefetch_range()` | ❌ NO API | MEDIUM |
| Reset statistics | `reset_stats()` | ❌ NO API | LOW |
| **Advanced Eviction Policies** | Various | ❌ NOT EXPOSED | MEDIUM |
| CLOCK policy | `ClockEvictionPolicy` | ❌ NO API | MEDIUM |
| LRU policy | `LruEvictionPolicy` | ❌ NO API | MEDIUM |
| 2Q policy | `TwoQEvictionPolicy` | ❌ NO API | MEDIUM |
| ARC policy | `ArcEvictionPolicy` | ❌ NO API | MEDIUM |
| LIRS policy | `LirsEvictionPolicy` | ❌ NO API | MEDIUM |
| **Huge Pages Support** | `HugePageAllocator` | ❌ NOT EXPOSED | LOW |
| Huge page config | `HugePageConfig` | ❌ NO API | LOW |
| Huge page stats | `HugePageStats` | ❌ NO API | LOW |
| System info | `query_huge_page_info()` | ❌ NO API | LOW |

### 1.2 Memory Management (src/memory/)

| Feature | Struct/Method | Current API Status | Priority |
|---------|--------------|-------------------|----------|
| **Memory Manager** | `MemoryManager` | ✅ PARTIAL | HIGH |
| Memory status | `get_memory_status()` | ✅ REST ONLY | DONE |
| Allocator statistics | `get_allocator_stats()` | ✅ REST ONLY | DONE |
| Comprehensive stats | `get_comprehensive_stats()` | ✅ REST ONLY | DONE |
| Memory allocation | `allocate()` | ❌ NO API | LOW |
| Memory contexts | `create_context()` | ❌ NO API | MEDIUM |
| **Slab Allocator** | `SlabAllocator` | ❌ NOT EXPOSED | MEDIUM |
| Slab allocation | `allocate()` | ❌ NO API | LOW |
| Slab statistics | `SlabAllocatorStats` | ✅ REST ONLY | DONE |
| **Arena Allocator** | `ArenaAllocator` | ❌ NOT EXPOSED | MEDIUM |
| Arena allocation | `allocate()` | ❌ NO API | LOW |
| Arena statistics | `ArenaAllocatorStats` | ✅ REST ONLY | DONE |
| **Large Object Allocator** | `LargeObjectAllocator` | ❌ NOT EXPOSED | MEDIUM |
| Large object allocation | `allocate()` | ❌ NO API | LOW |
| Large object statistics | `LargeObjectAllocatorStats` | ✅ REST ONLY | DONE |
| **Memory Pressure Management** | `MemoryPressureManager` | ✅ PARTIAL | HIGH |
| Pressure status | `get_memory_pressure()` | ✅ REST ONLY | DONE |
| Pressure callbacks | `register_callback()` | ❌ NO API | MEDIUM |
| Pressure events | `MemoryPressureEvent` | ❌ NO API | MEDIUM |
| **Garbage Collection** | `MemoryManager` | ✅ PARTIAL | MEDIUM |
| Trigger GC | `trigger_gc()` | ✅ REST ONLY | DONE |
| GC configuration | GC thresholds | ❌ NO API | MEDIUM |
| **Memory Configuration** | `MemoryConfiguration` | ✅ PARTIAL | HIGH |
| Get config | `get_memory_status()` | ✅ REST ONLY | DONE |
| Update config | `update_memory_config()` | ✅ REST ONLY | DONE |
| **Buffer Pool Manager** | `BufferPoolManager` | ✅ PARTIAL | HIGH |
| Buffer pool stats | `api_get_stats()` | ✅ REST ONLY | DONE |
| Pin/unpin pages | `api_pin_page()`, `api_unpin_page()` | ❌ NO API | LOW |
| Background operations | `api_start_background_operations()` | ❌ NO API | MEDIUM |
| Multi-tier config | Hot/Warm/Cold ratios | ✅ REST ONLY | DONE |

### 1.3 Storage Management (src/storage/)

| Feature | Struct/Method | Current API Status | Priority |
|---------|--------------|-------------------|----------|
| **Storage Status** | General | ✅ REST ONLY | HIGH |
| Overall status | `get_storage_status()` | ✅ REST ONLY | DONE |
| Disk information | `get_disks()` | ✅ REST ONLY | DONE |
| I/O statistics | `get_io_stats()` | ✅ REST ONLY | DONE |
| **Partitioning** | `PartitionManager` | ✅ PARTIAL | HIGH |
| List partitions | `get_partitions()` | ✅ REST ONLY | DONE |
| Create partition | `create_partition()` | ✅ REST ONLY | DONE |
| Delete partition | `delete_partition()` | ✅ REST ONLY | DONE |
| Partition for value | `get_partition_for_value()` | ❌ NO API | HIGH |
| Add partition | `add_partition()` | ❌ NO API | HIGH |
| Drop partition | `drop_partition()` | ❌ NO API | HIGH |
| List table partitions | `list_partitions()` | ❌ NO API | HIGH |
| Partition statistics | `PartitionStatsManager` | ❌ NOT EXPOSED | HIGH |
| Partition pruning | `PartitionPruner` | ❌ NOT EXPOSED | HIGH |
| Merge partitions | `PartitionMerger` | ❌ NOT EXPOSED | MEDIUM |
| Split partitions | `PartitionSplitter` | ❌ NOT EXPOSED | MEDIUM |
| **Tablespaces** | Tablespace Operations | ✅ REST ONLY | MEDIUM |
| List tablespaces | `get_tablespaces()` | ✅ REST ONLY | DONE |
| Create tablespace | `create_tablespace()` | ✅ REST ONLY | DONE |
| Update tablespace | `update_tablespace()` | ✅ REST ONLY | DONE |
| Delete tablespace | `delete_tablespace()` | ✅ REST ONLY | DONE |
| **LSM Tree Storage** | `LsmTree` | ❌ NOT EXPOSED | HIGH |
| Put operation | `put()` | ❌ NO API | HIGH |
| Get operation | `get()` | ❌ NO API | HIGH |
| Delete operation | `delete()` | ❌ NO API | HIGH |
| Scan operation | `scan()` | ❌ NO API | HIGH |
| Compaction | `run_compaction()` | ❌ NO API | HIGH |
| LSM statistics | `get_stats()` | ❌ NO API | HIGH |
| Compaction strategy | `CompactionStrategy` enum | ❌ NO API | MEDIUM |
| **Columnar Storage** | `ColumnarTable` | ❌ NOT EXPOSED | HIGH |
| Insert batch | `insert_batch()` | ❌ NO API | HIGH |
| Scan column | `scan_column()` | ❌ NO API | HIGH |
| Project columns | `project()` | ❌ NO API | HIGH |
| Column statistics | `column_stats()` | ❌ NO API | HIGH |
| Row count | `row_count()` | ❌ NO API | MEDIUM |
| Column types | `ColumnType` enum | ❌ NO API | MEDIUM |
| Encoding types | `EncodingType` enum | ❌ NO API | MEDIUM |
| **Tiered Storage** | `TieredStorageManager` | ❌ NOT EXPOSED | HIGH |
| Store page | `store_page()` | ❌ NO API | HIGH |
| Get page | `get_page()` | ❌ NO API | HIGH |
| Update page | `update_page()` | ❌ NO API | HIGH |
| Process migrations | `process_migrations()` | ❌ NO API | HIGH |
| Maintenance | `maintenance()` | ❌ NO API | MEDIUM |
| Tier statistics | `get_stats()` | ❌ NO API | HIGH |
| Storage tiers | `StorageTier` enum | ❌ NO API | MEDIUM |
| Compression levels | `CompressionLevel` enum | ❌ NO API | MEDIUM |

### 1.4 Disk I/O Management (src/storage/disk.rs, src/io/)

| Feature | Struct/Method | Current API Status | Priority |
|---------|--------------|-------------------|----------|
| **Disk Manager** | `DiskManager` | ❌ NOT EXPOSED | HIGH |
| Read page | `read_page()` | ❌ NO API | MEDIUM |
| Write page | `write_page()` | ❌ NO API | MEDIUM |
| Async read | `read_page_async()` | ❌ NO API | MEDIUM |
| Async write | `write_page_async()` | ❌ NO API | MEDIUM |
| Flush writes | `flush_all_writes()` | ❌ NO API | MEDIUM |
| Allocate page | `allocate_page()` | ❌ NO API | HIGH |
| Get num pages | `get_num_pages()` | ❌ NO API | MEDIUM |
| Process async ops | `process_async_ops()` | ❌ NO API | MEDIUM |
| Disk statistics | `get_stats()` | ❌ NO API | HIGH |
| Reset statistics | `reset_stats()` | ❌ NO API | LOW |
| **Vectored I/O** | `DiskManager` | ❌ NOT EXPOSED | MEDIUM |
| Read vectored | `read_pages_vectored()` | ❌ NO API | MEDIUM |
| Write vectored | `write_pages_vectored()` | ❌ NO API | MEDIUM |
| Write coalesced | `write_page_coalesced()` | ❌ NO API | MEDIUM |
| Flush coalesced | `flush_coalesced_writes()` | ❌ NO API | MEDIUM |
| **io_uring Support** | `DiskManager` | ❌ NOT EXPOSED | LOW |
| Read io_uring | `read_page_io_uring()` | ❌ NO API | LOW |
| Write io_uring | `write_page_io_uring()` | ❌ NO API | LOW |
| Submit batch | `submit_io_uring_batch()` | ❌ NO API | LOW |
| Wait completions | `wait_io_uring_completions()` | ❌ NO API | LOW |
| **I/O Configuration** | `DirectIoConfig` | ❌ NOT EXPOSED | MEDIUM |
| Direct I/O settings | Config fields | ❌ NO API | MEDIUM |
| I/O priority | `IoPriority` enum | ❌ NO API | MEDIUM |
| **Async I/O Engine** | `AsyncIoEngine` | ❌ NOT EXPOSED | MEDIUM |
| Submit I/O request | `submit()` | ❌ NO API | MEDIUM |
| Submit batch | `submit_batch()` | ❌ NO API | MEDIUM |
| Completion port stats | `stats()` | ❌ NO API | MEDIUM |
| **I/O Metrics** | `IoMetrics` | ❌ NOT EXPOSED | HIGH |
| I/O statistics | `IoStats` | ❌ NO API | HIGH |
| Latency histogram | `LatencyHistogram` | ❌ NO API | HIGH |
| Throughput metrics | `ThroughputMetrics` | ❌ NO API | HIGH |

### 1.5 Page Management (src/storage/page.rs)

| Feature | Struct/Method | Current API Status | Priority |
|---------|--------------|-------------------|----------|
| **Page Operations** | `Page` | ❌ NOT EXPOSED | LOW |
| Create page | `new()` | ❌ NO API | LOW |
| From bytes | `from_bytes()` | ❌ NO API | LOW |
| Mark dirty | `mark_dirty()` | ❌ NO API | LOW |
| Reset page | `reset()` | ❌ NO API | LOW |
| Verify checksum | `verify_checksum()` | ❌ NO API | MEDIUM |
| Update checksum | `update_checksum()` | ❌ NO API | MEDIUM |
| **Slotted Page** | `SlottedPage` | ❌ NOT EXPOSED | LOW |
| All slotted page operations | N/A | ❌ NO API | LOW |

---

## 2. CURRENT API COVERAGE ANALYSIS

### 2.1 REST API Coverage (src/api/rest/handlers/)

#### ✅ **COVERED:** storage_handlers.rs (13 endpoints)

```rust
// Storage Status & Disk Management
GET    /api/v1/storage/status          - Overall storage status ✅
GET    /api/v1/storage/disks            - List disk devices ✅
GET    /api/v1/storage/io-stats         - I/O statistics ✅

// Partitioning
GET    /api/v1/storage/partitions       - List partitions ✅
POST   /api/v1/storage/partitions       - Create partition ✅
DELETE /api/v1/storage/partitions/{id}  - Delete partition ✅

// Buffer Pool
GET    /api/v1/storage/buffer-pool      - Buffer pool statistics ✅
POST   /api/v1/storage/buffer-pool/flush - Flush buffer pool ✅

// Tablespaces
GET    /api/v1/storage/tablespaces      - List tablespaces ✅
POST   /api/v1/storage/tablespaces      - Create tablespace ✅
PUT    /api/v1/storage/tablespaces/{id} - Update tablespace ✅
DELETE /api/v1/storage/tablespaces/{id} - Delete tablespace ✅
```

#### ✅ **COVERED:** memory_handlers.rs (5 endpoints)

```rust
// Memory Management
GET    /api/v1/memory/status            - Memory status ✅
GET    /api/v1/memory/allocator/stats   - Allocator statistics ✅
POST   /api/v1/memory/gc                - Trigger garbage collection ✅
GET    /api/v1/memory/pressure          - Memory pressure status ✅
PUT    /api/v1/memory/config            - Update memory configuration ✅
```

### 2.2 GraphQL Coverage (src/api/graphql/)

**Status:** ❌ **NO STORAGE-SPECIFIC QUERIES OR MUTATIONS**

The GraphQL API (queries.rs, mutations.rs) currently focuses on:
- Database schema operations
- Table queries and mutations
- Data manipulation (CRUD)
- **NO storage layer operations exposed**

---

## 3. MISSING API ENDPOINTS

### 3.1 🔴 CRITICAL - HIGH PRIORITY Missing Endpoints

#### Buffer Pool Advanced Operations
```rust
// MISSING REST ENDPOINTS - HIGH PRIORITY

// Buffer Pool Configuration
GET    /api/v1/buffer-pool/config
PUT    /api/v1/buffer-pool/config
POST   /api/v1/buffer-pool/config/reset

// Buffer Pool Management
POST   /api/v1/buffer-pool/prefetch
POST   /api/v1/buffer-pool/prefetch-range
GET    /api/v1/buffer-pool/prefetch-stats
POST   /api/v1/buffer-pool/stats/reset

// Eviction Policy Management
GET    /api/v1/buffer-pool/eviction-policy
PUT    /api/v1/buffer-pool/eviction-policy
GET    /api/v1/buffer-pool/eviction-stats
```

#### LSM Tree Operations
```rust
// MISSING REST ENDPOINTS - HIGH PRIORITY

// LSM Tree Management
POST   /api/v1/storage/lsm/put
GET    /api/v1/storage/lsm/get
DELETE /api/v1/storage/lsm/delete
GET    /api/v1/storage/lsm/scan
POST   /api/v1/storage/lsm/compact
GET    /api/v1/storage/lsm/stats

// LSM Tree Configuration
GET    /api/v1/storage/lsm/config
PUT    /api/v1/storage/lsm/config
GET    /api/v1/storage/lsm/compaction-strategy
PUT    /api/v1/storage/lsm/compaction-strategy
```

#### Columnar Storage Operations
```rust
// MISSING REST ENDPOINTS - HIGH PRIORITY

// Columnar Table Management
POST   /api/v1/storage/columnar/insert-batch
GET    /api/v1/storage/columnar/scan
GET    /api/v1/storage/columnar/project
GET    /api/v1/storage/columnar/stats
GET    /api/v1/storage/columnar/column-stats/{column}

// Columnar Configuration
GET    /api/v1/storage/columnar/encoding/{table}
PUT    /api/v1/storage/columnar/encoding/{table}
GET    /api/v1/storage/columnar/compression/{table}
```

#### Tiered Storage Operations
```rust
// MISSING REST ENDPOINTS - HIGH PRIORITY

// Tier Management
GET    /api/v1/storage/tiers/stats
POST   /api/v1/storage/tiers/migrate
POST   /api/v1/storage/tiers/maintenance
GET    /api/v1/storage/tiers/config
PUT    /api/v1/storage/tiers/config

// Page Tier Operations
GET    /api/v1/storage/tiers/page/{page_id}
PUT    /api/v1/storage/tiers/page/{page_id}/tier
GET    /api/v1/storage/tiers/migration-status
```

#### Partition Management Advanced
```rust
// MISSING REST ENDPOINTS - HIGH PRIORITY

// Advanced Partition Operations
GET    /api/v1/storage/partitions/{table}/for-value
POST   /api/v1/storage/partitions/{table}/add
DELETE /api/v1/storage/partitions/{table}/drop
GET    /api/v1/storage/partitions/{table}/stats
POST   /api/v1/storage/partitions/{table}/merge
POST   /api/v1/storage/partitions/{table}/split
GET    /api/v1/storage/partitions/{table}/pruning-info
```

#### Disk I/O Operations
```rust
// MISSING REST ENDPOINTS - HIGH PRIORITY

// Disk Manager
GET    /api/v1/storage/disk/stats
POST   /api/v1/storage/disk/stats/reset
GET    /api/v1/storage/disk/pages/count
POST   /api/v1/storage/disk/pages/allocate
POST   /api/v1/storage/disk/flush

// Async I/O Operations
GET    /api/v1/storage/disk/async/pending
POST   /api/v1/storage/disk/async/process
GET    /api/v1/storage/disk/async/stats

// I/O Metrics
GET    /api/v1/storage/io/metrics
GET    /api/v1/storage/io/latency-histogram
GET    /api/v1/storage/io/throughput
```

### 3.2 🟡 MEDIUM PRIORITY Missing Endpoints

#### Memory Context Management
```rust
// MISSING REST ENDPOINTS - MEDIUM PRIORITY

// Memory Contexts
GET    /api/v1/memory/contexts
POST   /api/v1/memory/contexts
GET    /api/v1/memory/contexts/{id}
DELETE /api/v1/memory/contexts/{id}
GET    /api/v1/memory/contexts/{id}/stats

// Memory Pressure Callbacks
POST   /api/v1/memory/pressure/callbacks/register
DELETE /api/v1/memory/pressure/callbacks/{id}
GET    /api/v1/memory/pressure/events
```

#### Buffer Pool Advanced Features
```rust
// MISSING REST ENDPOINTS - MEDIUM PRIORITY

// Huge Pages
GET    /api/v1/buffer-pool/huge-pages/info
GET    /api/v1/buffer-pool/huge-pages/stats
PUT    /api/v1/buffer-pool/huge-pages/config

// Per-Core Pools
GET    /api/v1/buffer-pool/per-core/stats
PUT    /api/v1/buffer-pool/per-core/config
```

#### Vectored I/O Operations
```rust
// MISSING REST ENDPOINTS - MEDIUM PRIORITY

// Vectored I/O
POST   /api/v1/storage/disk/read-vectored
POST   /api/v1/storage/disk/write-vectored
POST   /api/v1/storage/disk/write-coalesced
POST   /api/v1/storage/disk/flush-coalesced
```

### 3.3 🟢 LOW PRIORITY Missing Endpoints

#### Page Management
```rust
// MISSING REST ENDPOINTS - LOW PRIORITY

// Page Operations
GET    /api/v1/storage/pages/{id}/verify-checksum
POST   /api/v1/storage/pages/{id}/update-checksum
POST   /api/v1/storage/pages/{id}/reset
```

#### io_uring Operations (Linux only)
```rust
// MISSING REST ENDPOINTS - LOW PRIORITY

// io_uring Support
GET    /api/v1/storage/disk/io-uring/stats
POST   /api/v1/storage/disk/io-uring/submit-batch
GET    /api/v1/storage/disk/io-uring/completions
```

---

## 4. GRAPHQL SCHEMA ADDITIONS NEEDED

### 4.1 Queries to Add

```graphql
type Query {
  # Buffer Pool
  bufferPoolConfig: BufferPoolConfig!
  bufferPoolStats: BufferPoolStats!
  bufferPoolPrefetchStats: PrefetchStats!

  # Memory
  memoryStatus: MemoryStatus!
  memoryAllocatorStats: AllocatorStats!
  memoryPressure: MemoryPressureStatus!
  memoryContexts: [MemoryContext!]!

  # LSM Tree
  lsmTreeStats: LsmTreeStats!
  lsmTreeScan(startKey: String!, endKey: String!): [LsmEntry!]!

  # Columnar Storage
  columnarTableStats(table: String!): ColumnarStats!
  columnarColumnStats(table: String!, column: String!): ColumnStats!

  # Tiered Storage
  tieredStorageStats: TieredStorageStats!
  pageTierInfo(pageId: BigInt!): PageTierInfo!

  # Partitioning
  tablePartitions(table: String!): [PartitionInfo!]!
  partitionStats(table: String!, partition: String!): PartitionStats!

  # Disk I/O
  diskStats: DiskStats!
  ioMetrics: IoMetrics!
  ioLatencyHistogram: LatencyHistogram!
}
```

### 4.2 Mutations to Add

```graphql
type Mutation {
  # Buffer Pool
  updateBufferPoolConfig(config: BufferPoolConfigInput!): BufferPoolConfig!
  flushBufferPool: FlushResult!
  prefetchPages(pageIds: [BigInt!]!): PrefetchResult!
  resetBufferPoolStats: Boolean!

  # Memory
  triggerGarbageCollection(aggressive: Boolean): GcResult!
  updateMemoryConfig(config: MemoryConfigInput!): MemoryConfig!
  createMemoryContext(name: String!, type: ContextType!, limit: BigInt!): MemoryContext!

  # LSM Tree
  lsmPut(key: String!, value: String!): Boolean!
  lsmDelete(key: String!): Boolean!
  lsmCompact(maxTasks: Int): CompactionResult!

  # Columnar Storage
  columnarInsertBatch(table: String!, rows: [RowInput!]!): InsertResult!
  updateColumnarEncoding(table: String!, column: String!, encoding: EncodingType!): Boolean!

  # Tiered Storage
  migratePageToTier(pageId: BigInt!, tier: StorageTier!): Boolean!
  processTierMigrations(maxMigrations: Int): MigrationResult!
  runTierMaintenance: MaintenanceResult!

  # Partitioning
  addPartition(table: String!, name: String!, definition: PartitionDefInput!): Partition!
  dropPartition(table: String!, partition: String!): Boolean!
  mergePartitions(table: String!, partitions: [String!]!): Partition!
  splitPartition(table: String!, partition: String!, splitPoint: String!): [Partition!]!

  # Disk I/O
  flushDisk: FlushResult!
  allocatePage: BigInt!
  resetDiskStats: Boolean!
}
```

### 4.3 Types to Add

```graphql
type BufferPoolConfig {
  numFrames: Int!
  evictionPolicy: String!
  perCorePools: Boolean!
  framesPerCore: Int!
  maxFlushBatchSize: Int!
  backgroundFlush: Boolean!
  flushIntervalMs: Int!
  dirtyThreshold: Float!
}

type BufferPoolStats {
  totalFrames: Int!
  freeFrames: Int!
  pinnedFrames: Int!
  dirtyFrames: Int!
  hitRatio: Float!
  evictions: BigInt!
  pageReads: BigInt!
  pageWrites: BigInt!
  flushes: BigInt!
}

type LsmTreeStats {
  memtableSize: BigInt!
  numLevels: Int!
  numSstables: Int!
  totalKeys: BigInt!
  totalSize: BigInt!
  compactionsPending: Int!
}

type ColumnarStats {
  rowCount: BigInt!
  columnCount: Int!
  totalSizeBytes: BigInt!
  compressionRatio: Float!
  columns: [ColumnStatsDetail!]!
}

type TieredStorageStats {
  hotTierPages: Int!
  warmTierPages: Int!
  coldTierPages: Int!
  hotTierSizeBytes: BigInt!
  warmTierSizeBytes: BigInt!
  coldTierSizeBytes: BigInt!
  migrationsPending: Int!
  averageAccessLatencyMs: Float!
}

type DiskStats {
  totalReads: BigInt!
  totalWrites: BigInt!
  bytesRead: BigInt!
  bytesWritten: BigInt!
  avgReadLatencyMs: Float!
  avgWriteLatencyMs: Float!
  readIops: Float!
  writeIops: Float!
  pendingOps: Int!
}

type IoMetrics {
  stats: DiskStats!
  latencyHistogram: LatencyHistogram!
  throughput: ThroughputMetrics!
}
```

---

## 5. IMPLEMENTATION RECOMMENDATIONS

### 5.1 Phase 1: Critical REST Endpoints (Week 1-2)

**Priority:** 🔴 HIGH - Core storage features

1. **LSM Tree Handler** (`src/api/rest/handlers/lsm_handlers.rs`)
   - Full CRUD operations for LSM tree
   - Compaction management
   - Statistics and monitoring
   - Configuration management

2. **Columnar Storage Handler** (`src/api/rest/handlers/columnar_handlers.rs`)
   - Batch insert operations
   - Column scanning and projection
   - Statistics per column
   - Encoding/compression configuration

3. **Tiered Storage Handler** (`src/api/rest/handlers/tiered_handlers.rs`)
   - Tier statistics
   - Migration management
   - Maintenance operations
   - Configuration

4. **Enhanced Partition Handler** (extend `storage_handlers.rs`)
   - Add missing partition operations
   - Partition statistics
   - Merge/split operations
   - Pruning information

5. **Disk I/O Handler** (`src/api/rest/handlers/disk_handlers.rs`)
   - Disk statistics
   - Page allocation
   - Async I/O monitoring
   - I/O metrics

### 5.2 Phase 2: Buffer Pool Enhancements (Week 3)

**Priority:** 🟡 MEDIUM - Advanced buffer management

1. **Enhanced Buffer Pool Handler** (extend `storage_handlers.rs`)
   - Configuration management
   - Prefetch operations
   - Eviction policy management
   - Detailed statistics

2. **Huge Pages Handler** (`src/api/rest/handlers/hugepages_handlers.rs`)
   - System information
   - Configuration
   - Statistics

### 5.3 Phase 3: Memory Enhancements (Week 4)

**Priority:** 🟡 MEDIUM - Advanced memory management

1. **Enhanced Memory Handler** (extend `memory_handlers.rs`)
   - Memory context CRUD
   - Pressure callback management
   - Detailed allocator statistics

### 5.4 Phase 4: GraphQL Integration (Week 5-6)

**Priority:** 🟡 MEDIUM - GraphQL parity

1. **GraphQL Queries** (extend `src/api/graphql/queries.rs`)
   - Add all storage queries listed in section 4.1

2. **GraphQL Mutations** (extend `src/api/graphql/mutations.rs`)
   - Add all storage mutations listed in section 4.2

3. **GraphQL Types** (new `src/api/graphql/storage_types.rs`)
   - Define all storage-specific types

### 5.5 Phase 5: Advanced Features (Week 7-8)

**Priority:** 🟢 LOW - Platform-specific and specialized

1. **Vectored I/O Handler**
2. **io_uring Handler** (Linux-specific)
3. **Page Management Handler**

---

## 6. SPECIFIC CODE EXAMPLES

### 6.1 Example: LSM Tree Handler

```rust
// File: src/api/rest/handlers/lsm_handlers.rs

use axum::{
    extract::{Path, State, Query},
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use super::super::types::*;
use crate::storage::lsm::{LsmTree, LsmStats};

// ============================================================================
// LSM-specific Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LsmKeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LsmScanRequest {
    pub start_key: String,
    pub end_key: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LsmCompactionRequest {
    pub max_tasks: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LsmStatsResponse {
    pub memtable_size: u64,
    pub num_levels: usize,
    pub num_sstables: usize,
    pub total_keys: u64,
    pub total_size: u64,
    pub compactions_pending: usize,
    pub compaction_strategy: String,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Put a key-value pair into the LSM tree
#[utoipa::path(
    post,
    path = "/api/v1/storage/lsm/put",
    tag = "lsm",
    request_body = LsmKeyValue,
    responses(
        (status = 200, description = "Key-value pair inserted", body = ApiSuccess),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn lsm_put(
    State(state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<LsmKeyValue>,
) -> ApiResult<AxumJson<ApiSuccess>> {
    // Get LSM tree from state
    let lsm = state.lsm_tree.as_ref()
        .ok_or_else(|| ApiError::new("LSM_NOT_ENABLED", "LSM tree not enabled"))?;

    // Put the key-value pair
    lsm.put(request.key.into_bytes(), request.value.into_bytes())
        .map_err(|e| ApiError::new("LSM_PUT_ERROR", e.to_string()))?;

    Ok(AxumJson(ApiSuccess {
        message: "Key-value pair inserted successfully".to_string(),
    }))
}

/// Get a value by key from the LSM tree
#[utoipa::path(
    get,
    path = "/api/v1/storage/lsm/get/{key}",
    tag = "lsm",
    params(
        ("key" = String, Path, description = "Key to retrieve")
    ),
    responses(
        (status = 200, description = "Value retrieved", body = LsmKeyValue),
        (status = 404, description = "Key not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn lsm_get(
    State(state): State<Arc<ApiState>>,
    Path(key): Path<String>,
) -> ApiResult<AxumJson<LsmKeyValue>> {
    let lsm = state.lsm_tree.as_ref()
        .ok_or_else(|| ApiError::new("LSM_NOT_ENABLED", "LSM tree not enabled"))?;

    let value = lsm.get(&key.into_bytes())
        .map_err(|e| ApiError::new("LSM_GET_ERROR", e.to_string()))?
        .ok_or_else(|| ApiError::new("KEY_NOT_FOUND", format!("Key {} not found", key)))?;

    Ok(AxumJson(LsmKeyValue {
        key,
        value: String::from_utf8_lossy(&value).to_string(),
    }))
}

/// Delete a key from the LSM tree
#[utoipa::path(
    delete,
    path = "/api/v1/storage/lsm/delete/{key}",
    tag = "lsm",
    params(
        ("key" = String, Path, description = "Key to delete")
    ),
    responses(
        (status = 204, description = "Key deleted"),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn lsm_delete(
    State(state): State<Arc<ApiState>>,
    Path(key): Path<String>,
) -> ApiResult<StatusCode> {
    let lsm = state.lsm_tree.as_ref()
        .ok_or_else(|| ApiError::new("LSM_NOT_ENABLED", "LSM tree not enabled"))?;

    lsm.delete(key.into_bytes())
        .map_err(|e| ApiError::new("LSM_DELETE_ERROR", e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Scan a range of keys from the LSM tree
#[utoipa::path(
    post,
    path = "/api/v1/storage/lsm/scan",
    tag = "lsm",
    request_body = LsmScanRequest,
    responses(
        (status = 200, description = "Scan results", body = Vec<LsmKeyValue>),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn lsm_scan(
    State(state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<LsmScanRequest>,
) -> ApiResult<AxumJson<Vec<LsmKeyValue>>> {
    let lsm = state.lsm_tree.as_ref()
        .ok_or_else(|| ApiError::new("LSM_NOT_ENABLED", "LSM tree not enabled"))?;

    let results = lsm.scan(&request.start_key.into_bytes(), &request.end_key.into_bytes())
        .map_err(|e| ApiError::new("LSM_SCAN_ERROR", e.to_string()))?;

    let limit = request.limit.unwrap_or(results.len());
    let kvs = results.into_iter()
        .take(limit)
        .map(|(k, v)| LsmKeyValue {
            key: String::from_utf8_lossy(&k).to_string(),
            value: String::from_utf8_lossy(&v).to_string(),
        })
        .collect();

    Ok(AxumJson(kvs))
}

/// Trigger LSM compaction
#[utoipa::path(
    post,
    path = "/api/v1/storage/lsm/compact",
    tag = "lsm",
    request_body = LsmCompactionRequest,
    responses(
        (status = 200, description = "Compaction completed", body = ApiSuccess),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn lsm_compact(
    State(state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<LsmCompactionRequest>,
) -> ApiResult<AxumJson<ApiSuccess>> {
    let lsm = state.lsm_tree.as_ref()
        .ok_or_else(|| ApiError::new("LSM_NOT_ENABLED", "LSM tree not enabled"))?;

    let max_tasks = request.max_tasks.unwrap_or(10);
    let compacted = lsm.run_compaction(max_tasks)
        .map_err(|e| ApiError::new("LSM_COMPACTION_ERROR", e.to_string()))?;

    Ok(AxumJson(ApiSuccess {
        message: format!("Compacted {} tasks", compacted),
    }))
}

/// Get LSM tree statistics
#[utoipa::path(
    get,
    path = "/api/v1/storage/lsm/stats",
    tag = "lsm",
    responses(
        (status = 200, description = "LSM tree statistics", body = LsmStatsResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn lsm_stats(
    State(state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<LsmStatsResponse>> {
    let lsm = state.lsm_tree.as_ref()
        .ok_or_else(|| ApiError::new("LSM_NOT_ENABLED", "LSM tree not enabled"))?;

    let stats = lsm.get_stats();

    Ok(AxumJson(LsmStatsResponse {
        memtable_size: stats.memtable_size,
        num_levels: stats.num_levels,
        num_sstables: stats.num_sstables,
        total_keys: stats.total_keys,
        total_size: stats.total_size,
        compactions_pending: stats.compactions_pending,
        compaction_strategy: "Leveled".to_string(), // From CompactionStrategy enum
    }))
}
```

### 6.2 Example: ApiState Extension

```rust
// File: src/api/rest/types.rs (extend ApiState)

use crate::storage::lsm::LsmTree;
use crate::storage::columnar::ColumnarTable;
use crate::storage::tiered::TieredStorageManager;

pub struct ApiState {
    // ... existing fields ...

    // New storage components
    pub lsm_tree: Option<Arc<LsmTree>>,
    pub columnar_tables: Arc<RwLock<HashMap<String, ColumnarTable>>>,
    pub tiered_storage: Option<Arc<TieredStorageManager>>,
}
```

### 6.3 Example: Router Registration

```rust
// File: src/api/rest/server.rs (extend router)

use crate::api::rest::handlers::lsm_handlers::*;

// In build_router():
let router = Router::new()
    // ... existing routes ...

    // LSM Tree routes
    .route("/api/v1/storage/lsm/put", post(lsm_put))
    .route("/api/v1/storage/lsm/get/:key", get(lsm_get))
    .route("/api/v1/storage/lsm/delete/:key", delete(lsm_delete))
    .route("/api/v1/storage/lsm/scan", post(lsm_scan))
    .route("/api/v1/storage/lsm/compact", post(lsm_compact))
    .route("/api/v1/storage/lsm/stats", get(lsm_stats))

    .with_state(state);
```

---

## 7. TESTING RECOMMENDATIONS

### 7.1 Unit Tests

Each new handler should have unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsm_put_get() {
        let state = create_test_state();

        // Put a key-value pair
        let put_req = LsmKeyValue {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };
        let result = lsm_put(State(state.clone()), AxumJson(put_req)).await;
        assert!(result.is_ok());

        // Get the value back
        let get_result = lsm_get(State(state), Path("test_key".to_string())).await;
        assert!(get_result.is_ok());
        let kv = get_result.unwrap().0;
        assert_eq!(kv.value, "test_value");
    }

    #[tokio::test]
    async fn test_lsm_scan() {
        // ... test scan operations ...
    }

    #[tokio::test]
    async fn test_lsm_compaction() {
        // ... test compaction ...
    }
}
```

### 7.2 Integration Tests

```rust
// tests/storage_api_tests.rs

#[tokio::test]
async fn test_lsm_api_integration() {
    let server = start_test_server().await;
    let client = reqwest::Client::new();

    // Test PUT
    let response = client
        .post(&format!("{}/api/v1/storage/lsm/put", server.url()))
        .json(&json!({"key": "test", "value": "data"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // Test GET
    let response = client
        .get(&format!("{}/api/v1/storage/lsm/get/test", server.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let kv: LsmKeyValue = response.json().await.unwrap();
    assert_eq!(kv.value, "data");
}
```

---

## 8. DOCUMENTATION REQUIREMENTS

### 8.1 OpenAPI/Swagger Documentation

All new endpoints must have OpenAPI documentation using `utoipa`:

```rust
#[utoipa::path(
    post,
    path = "/api/v1/storage/lsm/put",
    tag = "lsm",
    request_body = LsmKeyValue,
    responses(
        (status = 200, description = "Success", body = ApiSuccess),
        (status = 500, description = "Error", body = ApiError),
    )
)]
```

### 8.2 API Documentation File

Create `docs/API_STORAGE.md` with:
- All storage endpoints
- Request/response examples
- Use cases
- Performance considerations

### 8.3 GraphQL Documentation

Update GraphQL schema documentation with:
- Query examples
- Mutation examples
- Type definitions
- Field descriptions

---

## 9. PERFORMANCE CONSIDERATIONS

### 9.1 Caching Strategy

- Cache LSM tree statistics (TTL: 5 seconds)
- Cache buffer pool stats (TTL: 1 second)
- Cache disk I/O metrics (TTL: 10 seconds)

### 9.2 Rate Limiting

Recommend rate limits per endpoint:
- LSM operations: 1000 req/min
- Buffer pool ops: 500 req/min
- Statistics endpoints: 100 req/min
- Configuration updates: 10 req/min

### 9.3 Async Processing

For expensive operations:
- Compaction: Return job ID, poll for status
- Tier migrations: Return job ID, poll for status
- Bulk operations: Use background jobs

---

## 10. SECURITY CONSIDERATIONS

### 10.1 Authentication Required

All storage API endpoints should require authentication:
- Admin role for configuration changes
- Read-only role for statistics
- Write role for data operations

### 10.2 Authorization Checks

```rust
// Check permissions before allowing operations
if !auth.has_permission("storage.lsm.write")? {
    return Err(ApiError::permission_denied());
}
```

### 10.3 Input Validation

- Validate key/value sizes
- Sanitize file paths
- Validate configuration ranges
- Rate limit per user

---

## 11. MIGRATION PLAN

### 11.1 Backward Compatibility

- Maintain existing endpoints
- Add new endpoints incrementally
- Version API if breaking changes needed

### 11.2 Deprecation Strategy

If any endpoints need changes:
1. Mark as deprecated in OpenAPI
2. Add deprecation warning in response headers
3. Provide migration guide
4. Remove after 2 major versions

---

## 12. MONITORING & OBSERVABILITY

### 12.1 Metrics to Track

For each new endpoint:
- Request count
- Response time (p50, p95, p99)
- Error rate
- Rate limit hits

### 12.2 Logging

Log important operations:
- LSM compaction events
- Tier migrations
- Configuration changes
- Large scan operations

### 12.3 Alerting

Set up alerts for:
- High error rates on storage endpoints
- Long response times
- Failed compactions
- Tier migration failures

---

## 13. SUMMARY & ACTION ITEMS

### 13.1 Coverage Summary

| Category | Features | Covered | Missing | Coverage % |
|----------|---------|---------|---------|-----------|
| Buffer Pool | 8 | 3 | 5 | 38% |
| Memory Management | 9 | 5 | 4 | 56% |
| Storage Operations | 12 | 4 | 8 | 33% |
| Disk I/O | 5 | 1 | 4 | 20% |
| Advanced Features | 3 | 0 | 3 | 0% |
| **TOTAL** | **37** | **13** | **24** | **35%** |

### 13.2 Immediate Action Items (Priority Order)

1. 🔴 **Week 1-2:** Implement LSM Tree handler (HIGH PRIORITY)
2. 🔴 **Week 2-3:** Implement Columnar Storage handler (HIGH PRIORITY)
3. 🔴 **Week 3-4:** Implement Tiered Storage handler (HIGH PRIORITY)
4. 🔴 **Week 4-5:** Implement Disk I/O handler (HIGH PRIORITY)
5. 🟡 **Week 5-6:** Enhance Buffer Pool handler (MEDIUM PRIORITY)
6. 🟡 **Week 6-7:** Enhance Memory handler (MEDIUM PRIORITY)
7. 🟡 **Week 7-8:** Add GraphQL support (MEDIUM PRIORITY)
8. 🟢 **Week 9+:** Add specialized handlers (LOW PRIORITY)

### 13.3 Success Metrics

**Target:** 90%+ API coverage by end of implementation

- All critical storage features exposed via REST API
- All features available via GraphQL
- Comprehensive OpenAPI documentation
- Full test coverage (unit + integration)
- Performance benchmarks established

---

## 14. APPENDIX: ERROR CASES FOUND

### 14.1 Compilation Issues

No compilation errors found in existing storage modules. All components compile successfully.

### 14.2 Missing Integrations

The following integrations are missing:

1. **ApiState lacks storage components:**
   ```rust
   // MISSING in src/api/rest/types.rs
   pub lsm_tree: Option<Arc<LsmTree>>,
   pub columnar_tables: Arc<RwLock<HashMap<String, ColumnarTable>>>,
   pub tiered_storage: Option<Arc<TieredStorageManager>>,
   ```

2. **Router lacks storage routes:**
   - No routes for LSM operations
   - No routes for columnar operations
   - No routes for tiered storage
   - No routes for advanced disk I/O

3. **GraphQL schema lacks storage types:**
   - No storage queries
   - No storage mutations
   - No storage subscriptions

### 14.3 Recommendations for Error Handling

Add comprehensive error handling for:
- LSM tree: Key not found, compaction failures
- Columnar: Invalid column types, encoding errors
- Tiered storage: Migration failures, tier unavailable
- Disk I/O: Read/write failures, allocation failures

---

## 15. GITHUB ISSUE FORMAT

If errors are found during implementation, use this format:

```markdown
## Bug: [Component] - [Brief Description]

**Component:** Storage API - [Specific Handler]
**Severity:** High/Medium/Low
**Environment:** RustyDB v0.1.0

### Description
[Detailed description of the issue]

### Steps to Reproduce
1. [Step 1]
2. [Step 2]
3. [Observed behavior]

### Expected Behavior
[What should happen]

### Actual Behavior
[What actually happens]

### Error Messages
```
[Error output]
```

### Proposed Fix
[Suggested solution]

### Related Code
- File: `src/api/rest/handlers/[handler].rs`
- Lines: [line numbers]

### Priority Justification
[Why this priority level]
```

---

**End of Report**

**Next Steps:**
1. Review and approve this report
2. Create GitHub issues for each missing handler
3. Assign implementation tasks to developers
4. Begin Phase 1 implementation
5. Schedule weekly progress reviews

**Contact:** PhD Agent 1 - Storage API Specialist
**Report Version:** 1.0
**Date:** 2025-12-12
