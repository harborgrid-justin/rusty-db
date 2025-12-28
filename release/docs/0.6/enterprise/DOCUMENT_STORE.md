# RustyDB v0.6 Document Store (SODA-Compatible)

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: Application Developers, NoSQL Engineers

---

## Overview

RustyDB Document Store provides Oracle SODA (Simple Oracle Document Access) and MongoDB-compatible document database capabilities, enabling JSON/BSON document storage with rich querying, aggregation, and full-text search within a relational database.

**Code Base**: 7,108 lines across 9 modules
**Compatibility**: Oracle SODA API, MongoDB query syntax
**Status**: Backend complete (API exposure in progress)

### Key Features

- **Document Models**: JSON/BSON support with automatic UUID/auto-increment IDs
- **Query By Example (QBE)**: MongoDB-style operators ($eq, $gt, $in, $regex, etc.)
- **Aggregation Pipelines**: Full MongoDB aggregation ($match, $group, $project, $lookup)
- **Full-Text Search**: TF-IDF scoring with stemming and stop words
- **Change Streams**: Real-time notifications with resume tokens
- **SQL/JSON Integration**: Oracle-compatible JSON functions (JSON_TABLE, JSON_QUERY, etc.)
- **Indexes**: B-Tree, full-text, TTL, unique, partial, sparse

---

## Collections and Documents

### Create Collection

```javascript
// Create collection with schema validation
db.createCollection("users", {
  validator: {
    $jsonSchema: {
      required: ["name", "email"],
      properties: {
        name: { type: "string", minLength: 1, maxLength: 100 },
        email: { type: "string", pattern: "^[a-zA-Z0-9+_.-]+@[a-zA-Z0-9.-]+$" },
        age: { type: "integer", minimum: 0, maximum: 150 }
      }
    }
  }
});

// List collections
db.listCollections();

// Drop collection
db.dropCollection("users");
```

### Insert Documents

```javascript
// Insert single document (auto-generated UUID)
db.users.insert({
  name: "Alice Johnson",
  email: "alice@example.com",
  age: 30,
  role: "admin",
  created_at: new Date()
});

// Insert with custom ID
db.users.insert({
  _id: "user_001",
  name: "Bob Smith",
  email: "bob@example.com"
});

// Bulk insert
db.users.insertMany([
  { name: "Charlie", email: "charlie@example.com", age: 25 },
  { name: "Diana", email: "diana@example.com", age: 28 }
]);
```

---

## Query By Example (QBE)

### Comparison Operators

```javascript
// Equality
db.users.find({ age: 30 });
db.users.find({ age: { $eq: 30 } });

// Greater than / Less than
db.users.find({ age: { $gt: 25 } });
db.users.find({ age: { $gte: 25, $lt: 40 } });

// Not equal
db.users.find({ role: { $ne: "guest" } });

// In / Not in array
db.users.find({ role: { $in: ["admin", "moderator"] } });
db.users.find({ status: { $nin: ["banned", "deleted"] } });
```

### Logical Operators

```javascript
// AND
db.users.find({
  $and: [
    { age: { $gte: 25 } },
    { role: "admin" }
  ]
});

// OR
db.users.find({
  $or: [
    { role: "admin" },
    { role: "moderator" }
  ]
});

// NOT
db.users.find({
  age: { $not: { $lt: 18 } }
});

// NOR
db.users.find({
  $nor: [
    { status: "deleted" },
    { banned: true }
  ]
});
```

### Array Operators

```javascript
// Array contains element
db.posts.find({ tags: "javascript" });

// All elements match
db.posts.find({ tags: { $all: ["javascript", "tutorial"] } });

// Element match
db.posts.find({
  comments: {
    $elemMatch: {
      author: "Alice",
      rating: { $gte: 4 }
    }
  }
});

// Array size
db.posts.find({ tags: { $size: 3 } });
```

### Text Search and Regex

```javascript
// Regex match
db.users.find({ email: { $regex: ".*@gmail\\.com$" } });

// Case-insensitive
db.users.find({ name: { $regex: "alice", $options: "i" } });

// Exists check
db.users.find({ phone: { $exists: true } });

// Type check
db.users.find({ age: { $type: "integer" } });
```

---

## Aggregation Pipelines

### Basic Aggregation

```javascript
// Group and count
db.orders.aggregate([
  {
    $match: {
      status: "completed",
      order_date: { $gte: "2025-01-01" }
    }
  },
  {
    $group: {
      _id: "$customer_id",
      total_orders: { $sum: 1 },
      total_amount: { $sum: "$amount" },
      avg_amount: { $avg: "$amount" }
    }
  },
  {
    $sort: { total_amount: -1 }
  },
  {
    $limit: 10
  }
]);
```

### Advanced Aggregation

```javascript
// Join collections (lookup)
db.orders.aggregate([
  {
    $lookup: {
      from: "customers",
      localField: "customer_id",
      foreignField: "_id",
      as: "customer_info"
    }
  },
  {
    $unwind: "$customer_info"
  },
  {
    $project: {
      order_id: 1,
      amount: 1,
      customer_name: "$customer_info.name",
      customer_email: "$customer_info.email"
    }
  }
]);

// Faceted search
db.products.aggregate([
  {
    $facet: {
      by_category: [
        { $group: { _id: "$category", count: { $sum: 1 } } }
      ],
      by_price_range: [
        {
          $bucket: {
            groupBy: "$price",
            boundaries: [0, 50, 100, 200, 500],
            default: "500+"
          }
        }
      ],
      statistics: [
        {
          $group: {
            _id: null,
            avg_price: { $avg: "$price" },
            total_products: { $sum: 1 }
          }
        }
      ]
    }
  }
]);
```

---

## Full-Text Search

### Create Full-Text Index

```javascript
// Create full-text index
db.articles.createIndex(
  { title: "text", content: "text" },
  {
    name: "article_fulltext",
    weights: {
      title: 10,  // Title weighted 10x more than content
      content: 1
    },
    default_language: "english"
  }
);
```

### Search Documents

```javascript
// Basic text search
db.articles.find({
  $text: { $search: "database performance optimization" }
});

// Get relevance score
db.articles.find(
  { $text: { $search: "rust programming" } },
  { score: { $meta: "textScore" } }
).sort({ score: { $meta: "textScore" } });

// Phrase search
db.articles.find({
  $text: { $search: "\"machine learning\"" }  // Exact phrase
});

// Exclude terms
db.articles.find({
  $text: { $search: "database -oracle" }  // Contains "database" but not "oracle"
});
```

---

## Indexes

### Index Types

```javascript
// B-Tree index (single field)
db.users.createIndex({ email: 1 });

// Compound index
db.users.createIndex({ last_name: 1, first_name: 1 });

// Unique index
db.users.createIndex({ email: 1 }, { unique: true });

// Sparse index (only index non-null values)
db.users.createIndex({ phone: 1 }, { sparse: true });

// Partial index (conditional)
db.orders.createIndex(
  { customer_id: 1 },
  { partialFilterExpression: { status: "active" } }
);

// TTL index (auto-delete expired documents)
db.sessions.createIndex(
  { created_at: 1 },
  { expireAfterSeconds: 3600 }  // 1 hour
);
```

### Index Management

```javascript
// List indexes
db.users.getIndexes();

// Drop index
db.users.dropIndex("email_1");

// Rebuild index
db.users.reIndex("email_1");

// Index statistics
db.users.indexStats("email_1");
```

---

## Change Streams

### Watch for Changes

```javascript
// Watch all changes
const changeStream = db.users.watch();

changeStream.on('change', (event) => {
  console.log(event);
  // {
  //   operationType: 'insert',
  //   documentKey: { _id: '...' },
  //   fullDocument: { name: '...', email: '...' },
  //   timestamp: ...
  // }
});

// Watch specific operations
const updateStream = db.users.watch([
  { $match: { operationType: { $in: ['update', 'replace'] } } }
]);

// Resume from token (fault tolerance)
const resumeStream = db.users.watch([], {
  resumeAfter: previousResumeToken
});
```

### Change Types

- `insert`: Document inserted
- `update`: Document updated
- `replace`: Document replaced
- `delete`: Document deleted
- `drop`: Collection dropped

---

## JSONPath

### Query with JSONPath

```javascript
// Extract nested values
db.orders.find({}).select({
  order_id: "$.order_id",
  customer_name: "$.customer.name",
  first_item: "$.items[0].product_name",
  total: "$.summary.total"
});

// Filter with JSONPath
db.products.find({
  "$.attributes[?(@.name == 'color')].value": "red"
});

// Recursive descent
db.catalog.find({
  "$..price": { $gt: 100 }  // All nested price fields > 100
});

// Array slicing
db.posts.find({}).select({
  recent_comments: "$.comments[-5:]"  // Last 5 comments
});
```

---

## SQL/JSON Integration

### JSON Functions

```sql
-- Extract JSON as table
SELECT *
FROM JSON_TABLE(
  (SELECT data FROM documents WHERE id = 'doc1'),
  '$.items[*]'
  COLUMNS (
    product_name VARCHAR(100) PATH '$.name',
    quantity INTEGER PATH '$.qty',
    price DECIMAL(10,2) PATH '$.price'
  )
);

-- Query JSON
SELECT JSON_QUERY(data, '$.customer.address')
FROM orders
WHERE order_id = 'O-12345';

-- Extract scalar
SELECT JSON_VALUE(data, '$.customer.name')
FROM orders;

-- Check existence
SELECT order_id
FROM orders
WHERE JSON_EXISTS(data, '$.items[?(@.price > 100)]');

-- Aggregate JSON
SELECT JSON_OBJECTAGG(product_id, stock_count)
FROM inventory
GROUP BY warehouse_id;
```

---

## Document Versioning

### Enable Versioning

```javascript
db.createCollection("versioned_docs", {
  versioning: {
    enabled: true,
    max_versions: 10  // Keep last 10 versions
  }
});

// Insert creates version 1
db.versioned_docs.insert({
  _id: "doc1",
  title: "My Document",
  content: "Original content"
});

// Update creates version 2
db.versioned_docs.update(
  { _id: "doc1" },
  { $set: { content: "Updated content" } }
);

// View version history
db.versioned_docs.getVersionHistory("doc1");

// Restore previous version
db.versioned_docs.restoreVersion("doc1", version: 1);
```

---

## Performance Optimization

### Query Performance

```javascript
// Explain query plan
db.users.find({ age: { $gt: 25 } }).explain();

// Output shows index usage:
// {
//   index_used: "age_1",
//   index_scan: true,
//   documents_scanned: 150,
//   documents_returned: 150
// }

// Hint to force specific index
db.users.find({ age: 30 }).hint("age_1");
```

### SIMD Optimization

Full-text search uses SIMD tokenization for 4-8x performance improvement:

```javascript
// Automatic SIMD for:
// - Text tokenization
// - TF-IDF scoring
// - Distance calculations
```

### Statistics

```javascript
// Collection statistics
db.users.stats();

// Output:
// {
//   document_count: 10000,
//   avg_document_size: 512,
//   total_size_mb: 5.12,
//   index_count: 3,
//   index_size_mb: 1.2
// }

// Index statistics
db.users.indexStats();
```

---

## Best Practices

1. **Schema Validation**: Define JSON schemas for data integrity
2. **Indexes**: Index frequently queried fields
3. **Projection**: Select only needed fields to reduce bandwidth
4. **Aggregation**: Use pipelines instead of multiple queries
5. **TTL Indexes**: Auto-expire temporary data
6. **Change Streams**: Use resume tokens for fault tolerance
7. **Batch Operations**: Use bulk inserts/updates
8. **Document Size**: Keep documents < 16MB (BSON limit)
9. **Embedding vs. References**: Embed small subdocuments, reference large ones
10. **Full-Text Search**: Use for text-heavy queries instead of regex

---

**See Also**:
- [Document Store Test Report](/docs/DOCUMENT_STORE_TEST_REPORT.md)
- [Specialized Engines Flow](/diagrams/08_specialized_engines_flow.md)
- [JSON Functions Reference](../reference/JSON_FUNCTIONS.md)

**Document Version**: 1.0
**Last Updated**: December 2025
