# WebSocket Test Data for Storage Layer

This directory contains JSON test data files for testing WebSocket storage event streams in RustyDB.

## File Organization

| File | Event Type | Description |
|------|------------|-------------|
| `buffer_pool_events.json` | Buffer Pool | Page cache hit/miss, eviction, flush, pool statistics |
| `lsm_events.json` | LSM Tree | Memtable flush, compaction, level migration |
| `disk_io_events.json` | Disk I/O | Read/write operations, vectored I/O, I/O statistics |
| `tier_events.json` | Tiered Storage | Page migration between hot/warm/cold tiers, tier stats |
| `page_events.json` | Page Operations | Page allocation, split, merge, compaction, checksum failures |
| `columnar_events.json` | Columnar Storage | Batch inserts, column scans, encoding changes |

## Message Structure

All WebSocket messages follow this structure:

```json
{
  "message_type": "string",      // Event category (e.g., "buffer_pool_event")
  "data": {                       // Event-specific data (varies by type)
    "EventVariant": { ... }
  },
  "timestamp": 1702569600         // Unix timestamp (seconds since epoch)
}
```

## Event Types

### Buffer Pool Events

**PageHit**: Page found in buffer pool cache
```json
{
  "PageHit": {
    "page_id": 12345,
    "timestamp": 1702569600
  }
}
```

**PageMiss**: Page not found in cache, loaded from disk
```json
{
  "PageMiss": {
    "page_id": 67890,
    "timestamp": 1702569601
  }
}
```

**PageEvicted**: Page evicted from buffer pool
```json
{
  "PageEvicted": {
    "page_id": 11111,
    "reason": "LRU eviction - least recently used",
    "timestamp": 1702569602
  }
}
```

**PageFlushed**: Dirty page written to disk
```json
{
  "PageFlushed": {
    "page_id": 22222,
    "dirty_bytes": 4096,
    "timestamp": 1702569603
  }
}
```

**PoolStats**: Buffer pool statistics snapshot
```json
{
  "PoolStats": {
    "hit_rate": 0.95,
    "total_pages": 10000,
    "used_pages": 7500,
    "dirty_pages": 500,
    "timestamp": 1702569604
  }
}
```

### LSM Tree Events

**MemtableFlushed**: In-memory table flushed to disk as SSTable
```json
{
  "MemtableFlushed": {
    "tree_name": "user_data_lsm",
    "entries": 10000,
    "size_bytes": 102400,
    "timestamp": 1702569600
  }
}
```

**CompactionStarted**: Compaction process initiated
```json
{
  "CompactionStarted": {
    "tree_name": "user_data_lsm",
    "level": 0,
    "sstable_count": 4,
    "timestamp": 1702569605
  }
}
```

**CompactionCompleted**: Compaction finished
```json
{
  "CompactionCompleted": {
    "tree_name": "user_data_lsm",
    "level": 0,
    "old_sstables": 4,
    "new_sstables": 1,
    "duration_ms": 5432,
    "timestamp": 1702569610
  }
}
```

**LevelMigration**: SSTable migrated between levels
```json
{
  "LevelMigration": {
    "tree_name": "user_data_lsm",
    "from_level": 1,
    "to_level": 2,
    "sstable_count": 2,
    "timestamp": 1702569620
  }
}
```

### Disk I/O Events

**ReadCompleted**: Disk read operation finished
```json
{
  "ReadCompleted": {
    "page_id": 12345,
    "latency_us": 2500,
    "bytes": 4096,
    "timestamp": 1702569600
  }
}
```

**WriteCompleted**: Disk write operation finished
```json
{
  "WriteCompleted": {
    "page_id": 67890,
    "latency_us": 3200,
    "bytes": 4096,
    "timestamp": 1702569601
  }
}
```

**VectoredRead**: Batch read operation (multiple pages)
```json
{
  "VectoredRead": {
    "page_count": 10,
    "total_bytes": 40960,
    "latency_us": 5000,
    "timestamp": 1702569602
  }
}
```

**VectoredWrite**: Batch write operation (multiple pages)
```json
{
  "VectoredWrite": {
    "page_count": 8,
    "total_bytes": 32768,
    "latency_us": 6000,
    "timestamp": 1702569603
  }
}
```

**IoStats**: I/O statistics snapshot
```json
{
  "IoStats": {
    "reads_per_sec": 1500.0,
    "writes_per_sec": 800.0,
    "read_throughput_mbps": 150.5,
    "write_throughput_mbps": 120.3,
    "avg_read_latency_us": 2500,
    "avg_write_latency_us": 3200,
    "timestamp": 1702569604
  }
}
```

### Tiered Storage Events

**PageMigrated**: Page moved between storage tiers
```json
{
  "PageMigrated": {
    "page_id": 12345,
    "from_tier": "Hot",
    "to_tier": "Warm",
    "reason": "Access frequency below hot threshold",
    "timestamp": 1702569600
  }
}
```

Storage tiers: `"Hot"` (SSD/Memory), `"Warm"` (SSD), `"Cold"` (HDD/Cloud)

**TierStats**: Tier statistics snapshot
```json
{
  "TierStats": {
    "hot_pages": 1000,
    "warm_pages": 5000,
    "cold_pages": 10000,
    "total_migrations": 500,
    "avg_compression_ratio": 0.35,
    "bytes_saved": 50000000,
    "timestamp": 1702569630
  }
}
```

### Page Operation Events

**PageAllocated**: New page allocated
```json
{
  "PageAllocated": {
    "page_id": 12345,
    "size": 4096,
    "timestamp": 1702569600
  }
}
```

**PageSplit**: Page split into two pages
```json
{
  "PageSplit": {
    "original_page_id": 67890,
    "new_page_id": 67891,
    "reason": "Page utilization exceeded 90%",
    "timestamp": 1702569610
  }
}
```

**PageMerged**: Two pages merged into one
```json
{
  "PageMerged": {
    "page1_id": 11111,
    "page2_id": 11112,
    "result_page_id": 11111,
    "timestamp": 1702569620
  }
}
```

**PageCompacted**: Page defragmented
```json
{
  "PageCompacted": {
    "page_id": 22222,
    "bytes_reclaimed": 1024,
    "timestamp": 1702569630
  }
}
```

**ChecksumFailure**: Page checksum validation failed
```json
{
  "ChecksumFailure": {
    "page_id": 33333,
    "expected": 2863311530,
    "actual": 3735928559,
    "timestamp": 1702569640
  }
}
```

### Columnar Storage Events

**BatchInserted**: Batch of rows inserted
```json
{
  "BatchInserted": {
    "table_name": "analytics_data",
    "rows": 10000,
    "columns": 25,
    "timestamp": 1702569600
  }
}
```

**ColumnScanned**: Column scan operation completed
```json
{
  "ColumnScanned": {
    "table_name": "analytics_data",
    "column_name": "revenue",
    "rows_scanned": 1000000,
    "duration_ms": 543,
    "timestamp": 1702569610
  }
}
```

**EncodingChanged**: Column encoding strategy changed
```json
{
  "EncodingChanged": {
    "table_name": "analytics_data",
    "column_name": "status",
    "old_encoding": "Plain",
    "new_encoding": "Dictionary",
    "compression_ratio": 0.15,
    "timestamp": 1702569620
  }
}
```

Encoding types: `"Plain"`, `"Dictionary"`, `"RunLength"`, `"Delta"`, `"BitPacked"`

## Usage in Tests

### Loading Test Data

```rust
use serde_json::Value;
use std::fs;

#[test]
fn test_buffer_pool_events() {
    let data = fs::read_to_string("tests/test_data/websocket/buffer_pool_events.json")
        .expect("Failed to read test data");

    let events: Vec<Value> = serde_json::from_str(&data)
        .expect("Failed to parse JSON");

    assert_eq!(events.len(), 5);
    assert_eq!(events[0]["message_type"], "buffer_pool_event");
}
```

### WebSocket Integration Testing

```rust
use tokio_tungstenite::connect_async;
use futures_util::{SinkExt, StreamExt};

#[tokio::test]
async fn test_websocket_storage_events() {
    let url = "ws://localhost:8080/api/v1/ws/storage/buffer-pool";
    let (ws_stream, _) = connect_async(url).await.unwrap();

    let (mut write, mut read) = ws_stream.split();

    // Receive events
    while let Some(msg) = read.next().await {
        let msg = msg.unwrap();
        let event: Value = serde_json::from_str(&msg.to_string()).unwrap();

        assert!(event["message_type"].as_str().is_some());
        assert!(event["timestamp"].as_i64().is_some());
    }
}
```

## Event Frequency

- **Buffer Pool Events**: High frequency (10-100 Hz)
- **LSM Tree Events**: Medium frequency (0.1-1 Hz)
- **Disk I/O Events**: High frequency (100-1000 Hz)
- **Tier Events**: Low frequency (0.01-0.1 Hz)
- **Page Events**: Medium frequency (1-10 Hz)
- **Columnar Events**: Low frequency (0.1-1 Hz)

## WebSocket Endpoints

| Endpoint | Event Stream | Update Interval |
|----------|--------------|-----------------|
| `/api/v1/ws/storage/buffer-pool` | Buffer pool events | 100ms |
| `/api/v1/ws/storage/lsm` | LSM tree events | 1s |
| `/api/v1/ws/storage/io` | Disk I/O events | 100ms |
| `/api/v1/ws/storage/tiers` | Tier migration events | 5s |
| `/api/v1/ws/storage/pages` | Page operation events | 500ms |
| `/api/v1/ws/storage/columnar` | Columnar storage events | 1s |

## GraphQL Subscriptions

```graphql
subscription {
  bufferPoolMetrics(intervalSeconds: 1) {
    totalPages
    usedPages
    hitRate
    evictions
    timestamp
  }
}

subscription {
  lsmTreeEvents(treeName: "user_data_lsm") {
    eventType
    level
    sstableCount
    timestamp
  }
}

subscription {
  diskIoMetrics(intervalSeconds: 1) {
    readsPerSec
    writesPerSec
    iops
    avgLatency
    timestamp
  }
}

subscription {
  storageTierMetrics(intervalSeconds: 5) {
    hotPages
    warmPages
    coldPages
    totalMigrations
    timestamp
  }
}
```

## Performance Considerations

- Event batching recommended for high-frequency streams (>100 Hz)
- Client-side buffering needed for slow consumers
- Backpressure handling required for WebSocket connections
- Consider compression for high-volume event streams
- Use GraphQL subscriptions for filtered/aggregated data
- Raw WebSocket for real-time monitoring dashboards

## Related Files

- Event type definitions: `src/api/rest/handlers/storage_websocket_events.rs`
- WebSocket handlers: `src/api/rest/handlers/websocket_handlers.rs`
- GraphQL subscriptions: `src/api/graphql/subscriptions.rs`
- OpenAPI spec: `src/api/rest/openapi.rs`

## Changelog

- 2025-12-14: Initial test data creation with 6 event categories
- Event coverage: Buffer pool, LSM tree, Disk I/O, Tiered storage, Page ops, Columnar storage

---

**Created by**: Agent 1 - Storage Layer WebSocket Integration Specialist
**Date**: 2025-12-14
