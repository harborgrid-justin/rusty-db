# RustyDB v0.6.0 - Index Types Quick Reference

**Version**: 0.6.0 | **Updated**: December 28, 2025

---

## Index Types Overview

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

---

## Quick Command Reference

### Create Indexes (REST API)

```bash
# B+ Tree (default)
curl -X POST http://localhost:8080/api/indexes \
  -H "Content-Type: application/json" \
  -d '{"name":"idx_btree","type":"BPlusTree","columns":["id"]}'

# LSM Tree
curl -X POST http://localhost:8080/api/indexes \
  -H "Content-Type: application/json" \
  -d '{"name":"idx_lsm","type":"LSMTree","config":{"memtable_size":4194304}}'

# Hash Index
curl -X POST http://localhost:8080/api/indexes \
  -H "Content-Type: application/json" \
  -d '{"name":"idx_hash","type":"ExtendibleHash","bucket_capacity":64}'

# R-Tree (Spatial)
curl -X POST http://localhost:8080/api/indexes \
  -H "Content-Type: application/json" \
  -d '{"name":"idx_spatial","type":"Spatial","max_entries":8}'

# Full-Text
curl -X POST http://localhost:8080/api/indexes \
  -H "Content-Type: application/json" \
  -d '{"name":"idx_fts","type":"FullText","column":"content"}'

# Bitmap
curl -X POST http://localhost:8080/api/indexes \
  -H "Content-Type: application/json" \
  -d '{"name":"idx_bitmap","type":"Bitmap","column":"status"}'
```

### Create Indexes (SQL)

```sql
-- B+ Tree
CREATE INDEX idx_email ON users (email);

-- Multi-column
CREATE INDEX idx_name_email ON users (name, email);

-- Bitmap (low cardinality)
CREATE BITMAP INDEX idx_active ON users (active);

-- Expression index
CREATE INDEX idx_upper_email ON users (UPPER(email));

-- Partial index
CREATE INDEX idx_active_users ON users (email) WHERE active = true;
```

---

## When to Use Each Index Type

### B+ Tree
**Use For**:
- Range queries (BETWEEN, <, >)
- Sorted output (ORDER BY)
- Prefix matching (LIKE 'abc%')
- High cardinality columns

**Avoid For**:
- Exact equality only (use Hash instead)
- Very low cardinality (use Bitmap)

**Example**:
```sql
-- Good use cases
SELECT * FROM users WHERE age BETWEEN 25 AND 40;
SELECT * FROM products ORDER BY price DESC;
SELECT * FROM users WHERE email LIKE 'john%';

-- Create index
CREATE INDEX idx_age ON users (age);
CREATE INDEX idx_price ON products (price);
CREATE INDEX idx_email ON users (email);
```

---

### LSM Tree
**Use For**:
- Write-heavy workloads (INSERT/UPDATE)
- Append-only logs
- Time-series data
- Sequential writes

**Avoid For**:
- Random read-heavy workloads
- Frequent point lookups

**Example**:
```sql
-- Good use cases
INSERT INTO logs (timestamp, message) VALUES (NOW(), 'Event occurred');
INSERT INTO metrics (time, value) VALUES (NOW(), 42.5);

-- Create LSM index
CREATE LSM INDEX idx_logs ON logs (timestamp);
```

**Configuration**:
```json
{
  "memtable_size": 4194304,      // 4MB
  "max_levels": 7,
  "compaction_threshold": 4,
  "bloom_filter_size": 1048576   // 1MB
}
```

---

### Hash Index
**Use For**:
- Exact equality lookups (=)
- High cardinality keys
- Uniform distribution
- Point queries

**Avoid For**:
- Range queries
- Sorting (ORDER BY)
- Pattern matching (LIKE)

**Example**:
```sql
-- Good use cases
SELECT * FROM users WHERE user_id = 12345;
SELECT * FROM sessions WHERE session_token = 'abc123...';

-- Bad use cases
SELECT * FROM users WHERE age > 25;          -- No range support
SELECT * FROM users ORDER BY user_id;        -- No sorting
```

---

### R-Tree (Spatial)
**Use For**:
- Geospatial data (lat/lon)
- Bounding box queries
- Nearest neighbor (KNN)
- GIS applications

**Avoid For**:
- One-dimensional data
- Non-spatial queries

**Example**:
```sql
-- Good use cases
SELECT * FROM locations
WHERE lat BETWEEN 37.0 AND 38.0
  AND lon BETWEEN -122.0 AND -121.0;

-- Nearest 10 restaurants
SELECT * FROM restaurants
ORDER BY distance(lat, lon, 37.7749, -122.4194)
LIMIT 10;
```

**API Usage**:
```bash
# Spatial search
curl -X POST http://localhost:8080/api/indexes/idx_spatial/search \
  -H "Content-Type: application/json" \
  -d '{"bbox":{"min_x":0,"min_y":0,"max_x":10,"max_y":10}}'
```

---

### Full-Text Index
**Use For**:
- Text search
- Document retrieval
- Fuzzy matching
- Search engines

**Avoid For**:
- Exact string matching (use B+ Tree)
- Numeric data

**Example**:
```sql
-- Good use cases
SELECT * FROM documents WHERE MATCH(content, 'database performance');
SELECT * FROM articles WHERE content LIKE '%optimization%';

-- Create full-text index
CREATE FULLTEXT INDEX idx_content ON documents (content);
```

**API Usage**:
```bash
# Full-text search
curl "http://localhost:8080/api/indexes/idx_fts/search?q=database"
```

**Features**:
- TF-IDF scoring
- Stemming
- Phrase search
- Stop word filtering

---

### Bitmap Index
**Use For**:
- Low cardinality columns (<100 distinct values)
- Boolean columns
- Status/category fields
- Data warehouse queries

**Avoid For**:
- High cardinality columns
- Frequently updated columns

**Example**:
```sql
-- Good use cases (low cardinality)
CREATE BITMAP INDEX idx_status ON users (status);      -- active, inactive, pending
CREATE BITMAP INDEX idx_gender ON users (gender);      -- M, F, other
CREATE BITMAP INDEX idx_department ON employees (dept); -- Sales, IT, HR

-- Bad use cases (high cardinality)
-- CREATE BITMAP INDEX idx_email ON users (email);     -- ❌ Too many distinct values
```

**API Usage**:
```bash
# Bitmap AND operation
curl "http://localhost:8080/api/indexes/idx_bitmap/and?v1=active&v2=premium"
```

---

### Partial Index
**Use For**:
- Sparse conditions (e.g., only active users)
- Reduce index size
- Filter by predicate
- Subset indexing

**Avoid For**:
- Queries not matching predicate
- Full table coverage

**Example**:
```sql
-- Index only active users
CREATE INDEX idx_active_users ON users (email) WHERE active = true;

-- Index only high-value orders
CREATE INDEX idx_large_orders ON orders (order_date) WHERE amount > 1000;

-- Index only recent data
CREATE INDEX idx_recent_logs ON logs (timestamp)
WHERE timestamp > NOW() - INTERVAL '30 days';
```

**Benefits**:
- Smaller index size
- Faster index scans
- Lower maintenance cost

---

### Expression Index
**Use For**:
- Computed values
- Function-based queries
- Case-insensitive searches
- Derived columns

**Avoid For**:
- Simple column lookups

**Example**:
```sql
-- Case-insensitive email search
CREATE INDEX idx_upper_email ON users (UPPER(email));
SELECT * FROM users WHERE UPPER(email) = 'ALICE@EXAMPLE.COM';

-- Absolute value
CREATE INDEX idx_abs_balance ON accounts (ABS(balance));
SELECT * FROM accounts WHERE ABS(balance) > 1000;

-- Year extraction
CREATE INDEX idx_year ON orders (YEAR(order_date));
SELECT * FROM orders WHERE YEAR(order_date) = 2025;
```

---

## Index Operations

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
# Record query for analysis
curl -X POST http://localhost:8080/api/index-advisor/record \
  -H "Content-Type: application/json" \
  -d '{"query":{"table":"users","where_conditions":[{"column":"email","operator":"="}]}}'

# Get recommendations
curl http://localhost:8080/api/index-advisor/analyze
```

---

## Index Configuration

### B+ Tree Configuration
```json
{
  "order": 64,                    // Branching factor (32-256)
  "enable_simd": true,
  "enable_prefix_compression": true,
  "enable_bulk_load": true
}
```

### LSM Tree Configuration
```json
{
  "memtable_size": 4194304,       // 4MB
  "max_levels": 7,
  "compaction_threshold": 4,
  "bloom_filter_size": 1048576,   // 1MB
  "compression": "LZ4"
}
```

### Bloom Filter Configuration
```json
{
  "expected_items": 1000000,
  "false_positive_rate": 0.01     // 1%
}
```

---

## Index Selection Guide

### Decision Tree

```
Is it low cardinality (<100 values)?
├─ Yes → Bitmap Index
└─ No → Continue

Is it spatial data (lat/lon, geometry)?
├─ Yes → R-Tree Index
└─ No → Continue

Is it text search?
├─ Yes → Full-Text Index
└─ No → Continue

Is it write-heavy?
├─ Yes → LSM Tree Index
└─ No → Continue

Is it exact equality only?
├─ Yes → Hash Index
└─ No → Continue

Default → B+ Tree Index
```

### Quick Selection Table

| Query Pattern | Index Type | Example |
|---------------|------------|---------|
| `WHERE id = ?` | Hash or B+ Tree | User lookup |
| `WHERE age > ?` | B+ Tree | Range query |
| `WHERE status = ?` (few values) | Bitmap | Status filter |
| `WHERE created > ?` | B+ Tree | Time range |
| `WHERE UPPER(email) = ?` | Expression | Case-insensitive |
| `WHERE lat BETWEEN ? AND ?` | R-Tree | Location search |
| `WHERE content LIKE ?` | Full-Text | Text search |
| Heavy INSERTs | LSM Tree | Logging |

---

## Performance Tips

1. **Index Selectivity**: Higher selectivity = better index
2. **Composite Indexes**: Order columns by selectivity (high to low)
3. **Covering Indexes**: Include all query columns
4. **Index Size**: Smaller indexes = faster scans
5. **Maintenance**: Rebuild fragmented indexes
6. **Monitoring**: Watch index usage statistics

### Index Statistics

```bash
# Check index health
curl http://localhost:8080/api/indexes | jq '.[] | {name, type, size, usage}'

# Monitor LSM compaction
curl http://localhost:8080/api/indexes/idx_lsm/stats | jq '.level_stats'

# Bloom filter efficiency
curl http://localhost:8080/api/bloom-filters/bloom/stats | jq '{fpr, fill_ratio}'
```

---

## Common Issues

### Index Not Used
```sql
-- ❌ Problem: Function on indexed column
SELECT * FROM users WHERE UPPER(email) = 'TEST@EXAMPLE.COM';

-- ✅ Solution: Create expression index
CREATE INDEX idx_upper_email ON users (UPPER(email));
```

### Low Cardinality with B+ Tree
```sql
-- ❌ Problem: B+ Tree on boolean
CREATE INDEX idx_active ON users (is_active);  -- Only 2 values

-- ✅ Solution: Use bitmap index
CREATE BITMAP INDEX idx_active ON users (is_active);
```

### Write Amplification
```sql
-- ❌ Problem: Too many B+ Tree indexes on write-heavy table
-- Solution: Use LSM Tree for write-optimized workload
CREATE LSM INDEX idx_logs ON logs (timestamp);
```

---

## GraphQL Examples

### Create Index
```graphql
mutation {
  createIndex(input: {
    name: "idx_email"
    type: BTREE
    table: "users"
    columns: ["email"]
  }) {
    success
    message
  }
}
```

### Query Index Stats
```graphql
query {
  indexes {
    name
    type
    table
    columns
  }
  indexStats(name: "idx_email") {
    height
    totalKeys
    size
  }
}
```

### Drop Index
```graphql
mutation {
  dropIndex(name: "idx_email") {
    success
    message
  }
}
```

---

## Monitoring

### Check Index Health
```bash
# One-liner health check
curl -s http://localhost:8080/api/indexes | \
  jq '.[] | {name, type, size, usage}'
```

### Index Recommendations
```bash
# Get recommendations
curl -s http://localhost:8080/api/index-advisor/analyze | \
  jq '.recommendations[] | {type, table, columns, priority}'
```

### Performance Monitoring
```bash
# Monitor index usage
watch -n 5 'curl -s http://localhost:8080/api/indexes/idx_btree/stats | jq'
```

---

## Index Best Practices

1. **Don't Over-Index**: Each index has maintenance cost
2. **Monitor Usage**: Drop unused indexes
3. **Test Performance**: Measure before and after
4. **Consider Write Cost**: Indexes slow down writes
5. **Use Covering Indexes**: Reduce table lookups
6. **Rebuild Fragmented**: Periodic maintenance
7. **Choose Right Type**: Match index to query pattern

---

**Index Reference** | RustyDB v0.6.0 | Enterprise Database Server
