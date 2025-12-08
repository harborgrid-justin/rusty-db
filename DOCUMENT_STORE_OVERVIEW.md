# RustyDB Document Store Engine

## Overview

A comprehensive JSON Document Store Engine with Oracle SODA-like features, implemented in **7,060+ lines** of production-quality Rust code.

## Architecture

### Module Structure

```
src/document_store/
├── mod.rs                  (661 lines)  - Main module and DocumentStore API
├── document.rs             (719 lines)  - Document model with JSON/BSON support
├── collections.rs          (966 lines)  - Collection management
├── jsonpath.rs             (803 lines)  - JSONPath query engine
├── indexing.rs             (790 lines)  - Document indexing infrastructure
├── qbe.rs                  (775 lines)  - Query By Example (MongoDB-like)
├── aggregation.rs          (773 lines)  - Aggregation pipeline
├── changes.rs              (768 lines)  - Change streams
└── sql_json.rs             (805 lines)  - SQL/JSON integration
```

## Features Implemented

### 1. Document Model (`document.rs`)

**Core Features:**
- ✅ JSON document representation with `serde_json`
- ✅ BSON support for efficient binary storage
- ✅ Multiple ID generation strategies:
  - UUID v4 (default)
  - Auto-increment integers
  - Custom string IDs
- ✅ Document versioning with version history
- ✅ Document metadata (creation time, size, checksum, tags, TTL)
- ✅ Large document chunking (for documents > max size)
- ✅ Document builder pattern for fluent API

**Advanced Features:**
- SHA-256 content hashing for integrity
- TTL (Time-To-Live) with automatic expiration
- Custom metadata fields
- Document tagging system
- Version parent tracking

### 2. Collection Management (`collections.rs`)

**Core Features:**
- ✅ Create/drop collections
- ✅ Collection metadata and settings
- ✅ JSON Schema validation (draft-07 compatible)
- ✅ Collection statistics (document count, size, version distribution)
- ✅ Schema-based validation with multiple error handling modes

**Schema Validation:**
- Property type validation (string, number, integer, boolean, array, object)
- Required properties enforcement
- Min/max value constraints
- Min/max length constraints
- Pattern matching (regex)
- Enum value restrictions
- Min/max property count

**Settings:**
- ID generation strategy
- Versioning control
- Maximum document size limits
- Compression settings
- Default TTL
- Validation actions (error/warn)
- Audit logging

### 3. JSONPath Engine (`jsonpath.rs`)

**Core Features:**
- ✅ Full JSONPath implementation
- ✅ Root element access (`$`)
- ✅ Child element access (`.field`)
- ✅ Wildcard selection (`*`)
- ✅ Recursive descent (`..`)
- ✅ Array indexing (`[0]`, `[-1]`)
- ✅ Array slicing (`[start:end:step]`)
- ✅ Filter expressions (`[?(...)]`)
- ✅ Union operations (`[0,1,2]`)

**Filter Expression Support:**
- Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical operators: `&&`, `||`, `!`
- Regular expression matching (`=~`)
- Path references (`@.field`)
- Literal values (string, number, boolean, null)

### 4. Document Indexing (`indexing.rs`)

**Index Types:**
- ✅ B-Tree indexes (single and compound fields)
- ✅ Full-text search indexes (TF-IDF scoring)
- ✅ TTL indexes (automatic expiration)
- ✅ Unique indexes
- ✅ Partial indexes (filtered)
- ✅ Sparse indexes
- 🔄 Geospatial indexes (planned)

**Full-Text Search:**
- Inverted index with positional data
- TF-IDF scoring algorithm
- Phrase search support
- Configurable stop words
- Case sensitivity options
- Minimum word length filtering

**Index Features:**
- JSONPath-based field extraction
- Compound key support
- Range query optimization
- Automatic index maintenance

### 5. Query By Example (`qbe.rs`)

**Comparison Operators:**
- ✅ `$eq` - Equality
- ✅ `$ne` - Not equal
- ✅ `$gt` - Greater than
- ✅ `$gte` - Greater than or equal
- ✅ `$lt` - Less than
- ✅ `$lte` - Less than or equal

**Array Operators:**
- ✅ `$in` - Value in array
- ✅ `$nin` - Value not in array
- ✅ `$all` - All values match
- ✅ `$elemMatch` - Array element matching
- ✅ `$size` - Array size constraint

**Logical Operators:**
- ✅ `$and` - Logical AND
- ✅ `$or` - Logical OR
- ✅ `$nor` - Logical NOR
- ✅ `$not` - Logical NOT

**Other Operators:**
- ✅ `$exists` - Field existence check
- ✅ `$type` - Type checking
- ✅ `$regex` - Regular expression matching
- ✅ `$mod` - Modulo operation

**Projection:**
- Field inclusion/exclusion
- Computed field projections

**Geospatial Queries:**
- `$near` queries with distance limits
- Haversine distance calculations
- Min/max distance filters

### 6. Aggregation Pipeline (`aggregation.rs`)

**Pipeline Stages:**
- ✅ `$match` - Filter documents
- ✅ `$project` - Reshape documents
- ✅ `$group` - Group by key with accumulators
- ✅ `$sort` - Sort documents
- ✅ `$limit` - Limit result count
- ✅ `$skip` - Skip documents
- ✅ `$unwind` - Unwind arrays
- ✅ `$lookup` - Join collections (basic)
- ✅ `$facet` - Multi-faceted aggregation
- ✅ `$addFields` - Add computed fields
- ✅ `$count` - Count documents
- ✅ `$replaceRoot` - Replace document root

**Accumulators:**
- ✅ `$sum` - Sum values
- ✅ `$avg` - Average values
- ✅ `$min` - Minimum value
- ✅ `$max` - Maximum value
- ✅ `$count` - Count documents
- ✅ `$first` - First value
- ✅ `$last` - Last value
- ✅ `$push` - Push to array
- ✅ `$addToSet` - Add unique values to set

**Expressions:**
- `$add`, `$subtract`, `$multiply`, `$divide` - Arithmetic
- `$concat` - String concatenation
- `$cond` - Conditional expressions

### 7. Change Streams (`changes.rs`)

**Core Features:**
- ✅ Real-time change notifications
- ✅ Change event types (insert, update, delete, replace, drop, etc.)
- ✅ Resume tokens for stream resumption
- ✅ Filtered change streams
- ✅ Document diff generation
- ✅ Update description generation

**Event Types:**
- Insert
- Update
- Delete
- Replace
- Drop (collection)
- Rename
- DropDatabase
- Invalidate

**Features:**
- Cluster timestamp ordering
- Resume token encoding/decoding
- Event filtering by operation type, collection, or document ID
- Batch retrieval with configurable size
- Ring buffer for event history

**Diff Generation:**
- Field addition detection
- Field removal detection
- Field modification detection
- Nested object diffing
- Diff operation application

### 8. SQL/JSON Integration (`sql_json.rs`)

**Oracle-Compatible Functions:**
- ✅ `JSON_TABLE` - Convert JSON to relational table
- ✅ `JSON_QUERY` - Extract JSON fragments
- ✅ `JSON_VALUE` - Extract scalar values
- ✅ `JSON_EXISTS` - Check path existence
- ✅ `IS JSON` predicate - Validate JSON strings

**JSON Generation Functions:**
- ✅ `JSON_OBJECT` - Create JSON objects
- ✅ `JSON_ARRAY` - Create JSON arrays
- ✅ `JSON_OBJECTAGG` - Aggregate into objects
- ✅ `JSON_ARRAYAGG` - Aggregate into arrays
- ✅ `JSON_MERGEPATCH` - RFC 7396 merge patch
- ✅ `JSON_TRANSFORM` - Transform JSON documents

**JSON_TABLE Features:**
- Column path definitions
- Type conversion (string, integer, float, boolean, JSON, date, timestamp)
- Error handling (null, default, error)
- Empty value handling
- Nested path extraction

**Transform Operations:**
- Set value at path
- Remove path
- Rename field
- Keep only specified paths
- Remove specified paths

## API Examples

### Basic Document Operations

```rust
use rusty_db::document_store::{DocumentStore, Document, DocumentId};
use serde_json::json;

// Create store
let mut store = DocumentStore::new();
store.create_collection("users".to_string())?;

// Insert document
let doc = Document::from_json(
    DocumentId::new_uuid(),
    "users".to_string(),
    json!({
        "name": "Alice",
        "age": 30,
        "email": "alice@example.com"
    }),
)?;
let id = store.insert("users", doc)?;

// Find by ID
let doc = store.find_by_id("users", &id)?;

// Query documents
let results = store.find("users", json!({
    "age": {"$gte": 25}
}))?;

// Update document
store.update("users", &id, updated_doc)?;

// Delete document
store.delete("users", &id)?;
```

### Schema Validation

```rust
use rusty_db::document_store::collections::{JsonSchema, PropertySchema};

let mut schema = JsonSchema::new();
schema.add_required("name");
schema.add_required("email");
schema.add_property("name", PropertySchema::string().min_length(1));
schema.add_property("email", PropertySchema::string().pattern(r"^[\w\.-]+@[\w\.-]+\.\w+$"));
schema.add_property("age", PropertySchema::integer().minimum(0.0).maximum(150.0));

// Validate against schema
schema.validate(&json_doc)?;
```

### JSONPath Queries

```rust
use rusty_db::document_store::jsonpath::query;

let data = json!({
    "store": {
        "books": [
            {"title": "Book 1", "price": 10},
            {"title": "Book 2", "price": 20}
        ]
    }
});

// Simple path
let results = query("$.store.books[*].title", &data)?;

// Filter expression
let results = query("$.store.books[?(@.price > 15)]", &data)?;

// Recursive descent
let results = query("$..title", &data)?;
```

### Query By Example

```rust
use rusty_db::document_store::qbe::QueryBuilder;

let query = QueryBuilder::new()
    .eq("name", json!("Alice"))
    .gte("age", json!(18))
    .lt("age", json!(65))
    .in_array("status", vec![json!("active"), json!("premium")])
    .exists("email", true)
    .regex("city", "^New")
    .build();

let results = store.find("users", serde_json::to_value(query)?)?;
```

### Aggregation Pipeline

```rust
use rusty_db::document_store::aggregation::PipelineBuilder;
use std::collections::BTreeMap;

let mut sort_spec = BTreeMap::new();
sort_spec.insert("age".to_string(), -1); // Descending

let pipeline = PipelineBuilder::new()
    .match_stage(json!({"status": "active"}))
    .project(json!({"name": true, "age": true}))
    .sort(sort_spec)
    .limit(10)
    .build();

let results = store.aggregate("users", pipeline)?;
```

### Change Streams

```rust
use rusty_db::document_store::changes::{ChangeStreamFilter, ChangeEventType};

let filter = ChangeStreamFilter::new()
    .operation_types(vec![ChangeEventType::Insert, ChangeEventType::Update])
    .collections(vec!["users".to_string()]);

let mut cursor = store.watch(filter);

// Get changes
let changes = cursor.next_batch();
for change in changes {
    println!("Event: {:?} on {}", change.operation_type, change.collection);
}

// Resume from token
let token = cursor.get_resume_token().unwrap();
let mut new_cursor = store.watch(filter).resume_after(token);
```

### SQL/JSON Functions

```rust
use rusty_db::document_store::sql_json::{
    SqlJsonFunctions, JsonTableColumn, JsonDataType, JsonWrapper
};

// JSON_TABLE
let columns = vec![
    JsonTableColumn::new("title", "$.title", JsonDataType::String),
    JsonTableColumn::new("price", "$.price", JsonDataType::Float),
];
let result = store.json_table("books", &doc_id, "$.items[*]", columns)?;

// JSON_QUERY
let value = store.json_query("books", &doc_id, "$.items", JsonWrapper::WithWrapper)?;

// JSON_VALUE
let price = store.json_value("books", &doc_id, "$.items[0].price", JsonDataType::Float)?;

// JSON_EXISTS
let exists = store.json_exists("books", &doc_id, "$.items[0].isbn")?;
```

## Performance Characteristics

### Storage
- **Document size**: Up to 16 MB (configurable)
- **Chunking**: Automatic for large documents
- **Versioning**: O(1) version lookup
- **Metadata**: Compact representation with SHA-256 checksums

### Indexing
- **B-Tree**: O(log n) lookup and range queries
- **Full-text**: O(1) term lookup with TF-IDF scoring
- **Compound**: Support for multi-field indexes

### Queries
- **QBE**: O(n) document scan (O(log n) with indexes)
- **Aggregation**: Pipeline streaming with minimal memory overhead
- **JSONPath**: Optimized recursive descent with early termination

### Change Streams
- **Events**: Ring buffer with 10,000 event capacity (configurable)
- **Filtering**: O(1) event type filtering
- **Resume**: O(log n) resume token lookup

## Testing

All modules include comprehensive unit tests covering:
- Core functionality
- Edge cases
- Error handling
- Integration scenarios

Run tests with:
```bash
cargo test --lib document_store
```

## Dependencies

External crates used:
- `serde` / `serde_json` - JSON serialization
- `bson` - BSON support
- `uuid` - UUID generation
- `sha2` - SHA-256 hashing
- `regex` - Pattern matching

## Future Enhancements

Potential improvements:
- [ ] Persistent storage backend (currently in-memory)
- [ ] Geospatial index implementation
- [ ] Advanced full-text features (stemming, language-specific)
- [ ] Transaction support across multiple documents
- [ ] Horizontal sharding
- [ ] Replica sets
- [ ] Query optimization with statistics
- [ ] Compression (LZ4, Zstandard)

## License

Part of RustyDB - Enterprise-Grade Database Management System
