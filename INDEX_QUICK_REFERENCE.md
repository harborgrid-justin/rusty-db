# INDEX MODULE QUICK REFERENCE

## Index Types at a Glance

| Index Type | Best For | Complexity | Key Features |
|-----------|----------|------------|--------------|
| **B+ Tree** | Range queries, ordered data | O(log N) | SIMD search, adaptive order, bulk load |
| **LSM Tree** | Write-heavy workloads | O(1) writes | Compaction, bloom filters, tombstones |
| **Hash (Extendible)** | Point lookups | O(1) average | Dynamic growth, bucket splitting |
| **Hash (Linear)** | Incremental growth | O(1) average | No directory, gradual expansion |
| **R-Tree** | Spatial queries, GIS | O(log N) | Bounding boxes, KNN, quadratic split |
| **Full-Text** | Text search | O(k) | TF-IDF, stemming, phrase search |
| **Bitmap** | Low-cardinality columns | O(1) bitops | RLE compression, AND/OR/NOT |
| **Partial** | Filtered subsets | O(log N) | Predicate evaluation, space savings |
| **Expression** | Computed values | O(log N) | Function-based, UPPER/LOWER/ABS |
| **Swiss Table** | General hash map | O(1) | SIMD probes, 87.5% load factor |
| **Bloom Filter** | Membership tests | O(k) | SIMD, 100M ops/sec, ~1% FPR |

## Quick Command Reference

### Create Indexes
```bash
# B+ Tree
curl -X POST http://localhost:8080/api/indexes \
  -d '{"name":"idx_btree","type":"BPlusTree","columns":["id"]}'

# LSM Tree
curl -X POST http://localhost:8080/api/indexes \
  -d '{"name":"idx_lsm","type":"LSMTree","config":{"memtable_size":4194304}}'

# Hash Index
curl -X POST http://localhost:8080/api/indexes \
  -d '{"name":"idx_hash","type":"ExtendibleHash","bucket_capacity":64}'

# R-Tree
curl -X POST http://localhost:8080/api/indexes \
  -d '{"name":"idx_spatial","type":"Spatial","max_entries":8}'

# Full-Text
curl -X POST http://localhost:8080/api/indexes \
  -d '{"name":"idx_fts","type":"FullText","column":"content"}'

# Bitmap
curl -X POST http://localhost:8080/api/indexes \
  -d '{"name":"idx_bitmap","type":"Bitmap","column":"status"}'
```

### Query Operations
```bash
# Point search
curl "http://localhost:8080/api/indexes/idx_btree/search?key=123"

# Range scan
curl "http://localhost:8080/api/indexes/idx_btree/range?start=1&end=100"

# Full-text search
curl "http://localhost:8080/api/indexes/idx_fts/search?q=database"

# Spatial search
curl -X POST http://localhost:8080/api/indexes/idx_spatial/search \
  -d '{"bbox":{"min_x":0,"min_y":0,"max_x":10,"max_y":10}}'

# Bitmap AND
curl "http://localhost:8080/api/indexes/idx_bitmap/and?v1=active&v2=premium"
```

### Index Management
```bash
# List all indexes
curl http://localhost:8080/api/indexes

# Get statistics
curl http://localhost:8080/api/indexes/idx_btree/stats

# Drop index
curl -X DELETE http://localhost:8080/api/indexes/idx_btree
```

### Index Advisor
```bash
# Record query
curl -X POST http://localhost:8080/api/index-advisor/record \
  -d '{"query":{"table":"users","where_conditions":[{"column":"email","operator":"="}]}}'

# Get recommendations
curl http://localhost:8080/api/index-advisor/analyze
```

## Performance Guidelines

### When to Use Each Index Type

**B+ Tree**: 
- ✅ Range queries (BETWEEN, <, >)
- ✅ Sorted output (ORDER BY)
- ✅ Prefix matching (LIKE 'abc%')
- ❌ Exact equality only

**LSM Tree**:
- ✅ Write-heavy workloads (INSERT/UPDATE)
- ✅ Append-only logs
- ✅ Time-series data
- ❌ Random read-heavy

**Hash Index**:
- ✅ Exact equality (=)
- ✅ High cardinality keys
- ✅ Uniform distribution
- ❌ Range queries, sorting

**R-Tree**:
- ✅ Geospatial data (lat/lon)
- ✅ Bounding box queries
- ✅ Nearest neighbor (KNN)
- ❌ One-dimensional data

**Full-Text**:
- ✅ Text search
- ✅ Document retrieval
- ✅ Fuzzy matching
- ❌ Exact string matching

**Bitmap**:
- ✅ Low cardinality (<100 distinct values)
- ✅ Boolean columns
- ✅ Status/category fields
- ❌ High cardinality

**Partial**:
- ✅ Sparse conditions (e.g., only active users)
- ✅ Reduce index size
- ✅ Filter by predicate
- ❌ Full table scans

## File Locations
- Source: `/home/user/rusty-db/src/index/`
- Tests: `/home/user/rusty-db/tests/`
- Scripts: `/home/user/rusty-db/index_test_suite.sh`
- Report: `/home/user/rusty-db/INDEX_TEST_REPORT.md`

## Common Issues

### Index Not Used
```sql
-- Problem: Function on indexed column
SELECT * FROM users WHERE UPPER(email) = 'TEST@EXAMPLE.COM';

-- Solution: Use expression index
CREATE INDEX idx_upper_email ON users (UPPER(email));
```

### Low Cardinality
```sql
-- Problem: Hash index on boolean
CREATE INDEX idx_active ON users (is_active);  -- Bad

-- Solution: Use bitmap index
CREATE BITMAP INDEX idx_active ON users (is_active);  -- Good
```

### Write Amplification
```sql
-- Problem: Too many B-Tree indexes on write-heavy table
-- Solution: Use LSM Tree for write-optimized workload
CREATE LSM INDEX idx_logs ON logs (timestamp);
```

## Configuration Tuning

### B+ Tree
```json
{
  "order": 64,  // Branching factor (32-256)
  "enable_simd": true,
  "enable_prefix_compression": true
}
```

### LSM Tree
```json
{
  "memtable_size": 4194304,  // 4MB
  "max_levels": 7,
  "compaction_threshold": 4,
  "bloom_filter_size": 1048576  // 1MB
}
```

### Bloom Filter
```json
{
  "expected_items": 1000000,
  "false_positive_rate": 0.01  // 1%
}
```

## API Endpoints Summary

### REST API
- `POST /api/indexes` - Create
- `GET /api/indexes` - List
- `GET /api/indexes/{name}` - Get
- `DELETE /api/indexes/{name}` - Drop
- `GET /api/indexes/{name}/stats` - Statistics
- `POST /api/indexes/{name}/insert` - Insert
- `GET /api/indexes/{name}/search` - Search
- `GET /api/indexes/{name}/range` - Range scan
- `DELETE /api/indexes/{name}/delete/{key}` - Delete

### GraphQL
```graphql
query {
  indexes { name type table columns }
  indexStats(name: "idx_name") { height totalKeys }
}

mutation {
  createIndex(input: {name: "idx", type: BTREE}) { success }
  dropIndex(name: "idx") { success }
}
```

## Monitoring Queries

```bash
# Check index health
curl http://localhost:8080/api/indexes | jq '.[] | {name, type, size, usage}'

# Get index recommendations
curl http://localhost:8080/api/index-advisor/analyze | jq '.recommendations[] | {type, table, columns, priority}'

# Monitor LSM compaction
curl http://localhost:8080/api/indexes/idx_lsm/stats | jq '.level_stats'

# Bloom filter efficiency
curl http://localhost:8080/api/bloom-filters/bloom/stats | jq '{fpr, fill_ratio, memory_mb: (.memory_bytes / 1048576)}'
```

---
**Version**: 1.0
**Last Updated**: 2025-12-11
**Agent**: Enterprise Index Testing Agent
