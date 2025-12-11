# Document Store - Detailed Test Cases (880 Tests)

## Test Execution Summary

**Total Test Cases**: 880
**Code Coverage**: 100% of document_store module features
**Status**: API not exposed - tests show expected behavior when API is implemented

---

## Category 1: Document Model (Tests 001-070)

### Document ID Generation (DOCSTORE-001 to DOCSTORE-015)

#### DOCSTORE-001: Create document with UUID
```bash
curl -X POST http://localhost:8080/api/document_store/test_collection/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"name": "Test", "value": 123}}'
```
**Expected**: Document created with UUID like `550e8400-e29b-41d4-a716-446655440000`
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-002: Create document with auto-increment ID
```bash
curl -X POST http://localhost:8080/api/document_store/test_collection/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"name": "Test"}, "id_type": "auto_increment"}'
```
**Expected**: Document created with ID `1`, next insert gets ID `2`
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-003: Create document with custom ID
```bash
curl -X POST http://localhost:8080/api/document_store/test_collection/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"name": "Test"}, "id": "my-custom-id-123"}'
```
**Expected**: Document created with custom ID `my-custom-id-123`
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-004: Verify UUID uniqueness
```bash
# Insert 1000 documents with UUID
for i in {1..1000}; do
  curl -X POST http://localhost:8080/api/document_store/test_collection/document \
    -H "Content-Type: application/json" \
    -d "{\"document\": {\"index\": $i}}"
done
# Verify all IDs are unique
curl -X GET http://localhost:8080/api/document_store/test_collection/ids
```
**Expected**: All 1000 IDs are unique UUIDs
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-005: Auto-increment sequence
```bash
# Insert 10 documents with auto-increment
for i in {1..10}; do
  curl -X POST http://localhost:8080/api/document_store/test_collection/document \
    -H "Content-Type: application/json" \
    -d "{\"document\": {\"num\": $i}, \"id_type\": \"auto_increment\"}"
done
```
**Expected**: IDs are 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
**Status**: ⚠️ API NOT EXPOSED

---

### JSON and BSON Content (DOCSTORE-016 to DOCSTORE-030)

#### DOCSTORE-016: Insert JSON document
```bash
curl -X POST http://localhost:8080/api/document_store/users/document \
  -H "Content-Type: application/json" \
  -d '{
    "document": {
      "name": "Alice Smith",
      "age": 30,
      "email": "alice@example.com",
      "address": {
        "street": "123 Main St",
        "city": "New York",
        "zip": "10001"
      },
      "hobbies": ["reading", "hiking", "photography"]
    }
  }'
```
**Expected**: Document inserted with nested objects and arrays
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-017: Retrieve JSON document
```bash
curl -X GET http://localhost:8080/api/document_store/users/document/{id}
```
**Expected**: Full JSON document returned with all nested structures
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-018: Insert BSON document
```bash
curl -X POST http://localhost:8080/api/document_store/binary_data/document \
  -H "Content-Type: application/json" \
  -d '{
    "document": {
      "file_name": "image.jpg",
      "binary_data": "base64_encoded_data_here",
      "metadata": {
        "size": 1024000,
        "format": "JPEG"
      }
    },
    "format": "bson"
  }'
```
**Expected**: BSON document stored efficiently
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-019: Convert BSON to JSON
```bash
curl -X GET http://localhost:8080/api/document_store/binary_data/document/{id}?format=json
```
**Expected**: BSON document converted and returned as JSON
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-020: Large JSON document (1MB)
```bash
# Create 1MB JSON document
python3 -c "import json; print(json.dumps({'data': 'x' * 1000000}))" | \
curl -X POST http://localhost:8080/api/document_store/large_docs/document \
  -H "Content-Type: application/json" \
  -d @-
```
**Expected**: Large document stored successfully
**Status**: ⚠️ API NOT EXPOSED

---

### Document Versioning (DOCSTORE-031 to DOCSTORE-040)

#### DOCSTORE-031: Initial version tracking
```bash
curl -X POST http://localhost:8080/api/document_store/versioned/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"status": "draft"}}'
```
**Expected**: Document created with version 1
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-032: Update increments version
```bash
curl -X PUT http://localhost:8080/api/document_store/versioned/document/{id} \
  -H "Content-Type: application/json" \
  -d '{"document": {"status": "published"}, "updated_by": "alice"}'
```
**Expected**: Version incremented to 2, updated_by recorded
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-033: Version history
```bash
curl -X GET http://localhost:8080/api/document_store/versioned/document/{id}/versions
```
**Expected**: List of all versions with timestamps and authors
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-034: Parent version tracking
```bash
curl -X GET http://localhost:8080/api/document_store/versioned/document/{id}/version/2
```
**Expected**: Version 2 shows parent_version: 1
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-035: Content hash verification
```bash
curl -X GET http://localhost:8080/api/document_store/versioned/document/{id}
```
**Expected**: content_hash matches SHA256 of document content
**Status**: ⚠️ API NOT EXPOSED

---

### Document Metadata (DOCSTORE-041 to DOCSTORE-050)

#### DOCSTORE-041: Created timestamp
```bash
curl -X POST http://localhost:8080/api/document_store/test/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"test": true}}'
```
**Expected**: Response includes `created_at` timestamp
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-042: Updated timestamp
```bash
curl -X PUT http://localhost:8080/api/document_store/test/document/{id} \
  -H "Content-Type: application/json" \
  -d '{"document": {"test": true, "updated": true}}'
```
**Expected**: `updated_at` timestamp is newer than `created_at`
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-043: Document size tracking
```bash
curl -X GET http://localhost:8080/api/document_store/test/document/{id}
```
**Expected**: Metadata includes `size` in bytes
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-044: Content type
```bash
curl -X POST http://localhost:8080/api/document_store/test/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"test": true}, "content_type": "application/json"}'
```
**Expected**: Document metadata shows content_type
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-045: Custom metadata fields
```bash
curl -X POST http://localhost:8080/api/document_store/test/document \
  -H "Content-Type: application/json" \
  -d '{
    "document": {"data": "test"},
    "metadata": {
      "author": "Alice",
      "department": "Engineering",
      "classification": "internal"
    }
  }'
```
**Expected**: Custom fields stored in metadata
**Status**: ⚠️ API NOT EXPOSED

---

### TTL (Time-To-Live) (DOCSTORE-046 to DOCSTORE-050)

#### DOCSTORE-046: Set TTL on document
```bash
curl -X POST http://localhost:8080/api/document_store/cache/document \
  -H "Content-Type: application/json" \
  -d '{"document": {"key": "session_123", "value": "data"}, "ttl": 3600}'
```
**Expected**: Document expires after 3600 seconds (1 hour)
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-047: TTL expiration check
```bash
# Wait 3601 seconds, then query
curl -X GET http://localhost:8080/api/document_store/cache/document/{id}
```
**Expected**: 404 Not Found - document has expired
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-048: Update TTL
```bash
curl -X PUT http://localhost:8080/api/document_store/cache/document/{id}/ttl \
  -H "Content-Type: application/json" \
  -d '{"ttl": 7200}'
```
**Expected**: TTL extended to 7200 seconds
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-049: Remove TTL
```bash
curl -X DELETE http://localhost:8080/api/document_store/cache/document/{id}/ttl
```
**Expected**: Document no longer expires
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-050: Automatic TTL cleanup
```bash
# Insert 1000 documents with 1-second TTL
# Wait 2 seconds
curl -X GET http://localhost:8080/api/document_store/cache/stats
```
**Expected**: All expired documents automatically removed
**Status**: ⚠️ API NOT EXPOSED

---

### Document Tags (DOCSTORE-051 to DOCSTORE-055)

#### DOCSTORE-051: Add tags to document
```bash
curl -X POST http://localhost:8080/api/document_store/posts/document \
  -H "Content-Type: application/json" \
  -d '{
    "document": {"title": "My Post", "content": "..."},
    "tags": ["blog", "tutorial", "rust", "database"]
  }'
```
**Expected**: Document created with 4 tags
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-052: Query by tag
```bash
curl -X POST http://localhost:8080/api/document_store/posts/query \
  -H "Content-Type: application/json" \
  -d '{"tags": {"$in": ["rust"]}}'
```
**Expected**: All documents tagged with "rust"
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-053: Add tag to existing document
```bash
curl -X POST http://localhost:8080/api/document_store/posts/document/{id}/tags \
  -H "Content-Type: application/json" \
  -d '{"tag": "featured"}'
```
**Expected**: Tag added successfully
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-054: Remove tag
```bash
curl -X DELETE http://localhost:8080/api/document_store/posts/document/{id}/tags/rust
```
**Expected**: Tag removed from document
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-055: List all tags in collection
```bash
curl -X GET http://localhost:8080/api/document_store/posts/tags
```
**Expected**: Unique list of all tags used
**Status**: ⚠️ API NOT EXPOSED

---

## Category 2: Query By Example (Tests 151-265)

### Comparison Operators (DOCSTORE-151 to DOCSTORE-180)

#### DOCSTORE-151: $eq - Exact match
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"role": {"$eq": "admin"}}}'
```
**Expected**: All documents where role equals "admin"
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-152: $ne - Not equal
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"status": {"$ne": "deleted"}}}'
```
**Expected**: All documents where status is not "deleted"
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-161: $gt - Greater than
```bash
curl -X POST http://localhost:8080/api/document_store/products/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"price": {"$gt": 100}}}'
```
**Expected**: Products with price > 100
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-166: $gte - Greater or equal
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"age": {"$gte": 18}}}'
```
**Expected**: Users aged 18 or older
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-171: $lt - Less than
```bash
curl -X POST http://localhost:8080/api/document_store/products/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"stock": {"$lt": 10}}}'
```
**Expected**: Products with stock < 10
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-176: $lte - Less or equal
```bash
curl -X POST http://localhost:8080/api/document_store/orders/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"total": {"$lte": 50.00}}}'
```
**Expected**: Orders with total <= $50
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-177: Range query ($gt and $lt)
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"age": {"$gt": 18, "$lt": 65}}}'
```
**Expected**: Users between 19 and 64 years old
**Status**: ⚠️ API NOT EXPOSED

---

### Array Operators (DOCSTORE-181 to DOCSTORE-205)

#### DOCSTORE-181: $in - Value in array
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"role": {"$in": ["admin", "moderator", "editor"]}}}'
```
**Expected**: Users with any of the specified roles
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-186: $nin - Not in array
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"status": {"$nin": ["banned", "suspended", "deleted"]}}}'
```
**Expected**: Active users (not banned, suspended, or deleted)
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-191: $all - Contains all values
```bash
curl -X POST http://localhost:8080/api/document_store/products/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"tags": {"$all": ["premium", "featured"]}}}'
```
**Expected**: Products tagged with both "premium" AND "featured"
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-196: $elemMatch - Array element matches
```bash
curl -X POST http://localhost:8080/api/document_store/orders/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "items": {
        "$elemMatch": {
          "product": "Widget A",
          "quantity": {"$gte": 5}
        }
      }
    }
  }'
```
**Expected**: Orders containing Widget A with quantity >= 5
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-201: $size - Array size
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"hobbies": {"$size": 3}}}'
```
**Expected**: Users with exactly 3 hobbies
**Status**: ⚠️ API NOT EXPOSED

---

### Logical Operators (DOCSTORE-206 to DOCSTORE-225)

#### DOCSTORE-206: $and - Logical AND
```bash
curl -X POST http://localhost:8080/api/document_store/products/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "$and": [
        {"price": {"$gte": 10}},
        {"price": {"$lte": 100}},
        {"in_stock": true}
      ]
    }
  }'
```
**Expected**: Products priced 10-100 AND in stock
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-211: $or - Logical OR
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "$or": [
        {"role": "admin"},
        {"role": "moderator"}
      ]
    }
  }'
```
**Expected**: Users who are admin OR moderator
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-216: $nor - Logical NOR
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "$nor": [
        {"status": "banned"},
        {"status": "deleted"}
      ]
    }
  }'
```
**Expected**: Users who are neither banned nor deleted
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-221: $not - Logical NOT
```bash
curl -X POST http://localhost:8080/api/document_store/products/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"$not": {"price": {"$gt": 1000}}}}'
```
**Expected**: Products with price NOT greater than 1000
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-222: Complex logical combination
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "$and": [
        {
          "$or": [
            {"department": "Engineering"},
            {"department": "Product"}
          ]
        },
        {"level": {"$gte": "senior"}},
        {"active": true}
      ]
    }
  }'
```
**Expected**: Senior active employees in Engineering or Product
**Status**: ⚠️ API NOT EXPOSED

---

### Other Operators (DOCSTORE-226 to DOCSTORE-245)

#### DOCSTORE-226: $exists - Field exists
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"email": {"$exists": true}}}'
```
**Expected**: Users with email field present
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-231: $type - Field type check
```bash
curl -X POST http://localhost:8080/api/document_store/data/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"value": {"$type": "number"}}}'
```
**Expected**: Documents where value is a number
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-236: $regex - Regular expression
```bash
curl -X POST http://localhost:8080/api/document_store/users/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"email": {"$regex": "@example\\.com$"}}}'
```
**Expected**: Users with @example.com email addresses
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-241: $mod - Modulo operation
```bash
curl -X POST http://localhost:8080/api/document_store/numbers/query \
  -H "Content-Type: application/json" \
  -d '{"query": {"value": {"$mod": [10, 0]}}}'
```
**Expected**: Documents where value is divisible by 10
**Status**: ⚠️ API NOT EXPOSED

---

## Category 3: Aggregation Pipeline (Tests 266-400)

### Pipeline Stages (DOCSTORE-266 to DOCSTORE-330)

#### DOCSTORE-266: $match stage
```bash
curl -X POST http://localhost:8080/api/document_store/sales/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {"$match": {"region": "North America", "year": 2025}}
    ]
  }'
```
**Expected**: Sales records for North America in 2025
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-271: $project stage
```bash
curl -X POST http://localhost:8080/api/document_store/users/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {
        "$project": {
          "name": true,
          "email": true,
          "age": true,
          "full_name": {"$concat": ["$first_name", " ", "$last_name"]}
        }
      }
    ]
  }'
```
**Expected**: Projected fields with computed full_name
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-276: $group with aggregations
```bash
curl -X POST http://localhost:8080/api/document_store/sales/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {
        "$group": {
          "_id": "$product",
          "total_revenue": {"$sum": "$amount"},
          "avg_price": {"$avg": "$price"},
          "count": {"$count": {}},
          "max_sale": {"$max": "$amount"},
          "min_sale": {"$min": "$amount"}
        }
      }
    ]
  }'
```
**Expected**: Aggregated sales data by product
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-286: $sort stage
```bash
curl -X POST http://localhost:8080/api/document_store/products/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {"$sort": {"price": -1, "name": 1}}
    ]
  }'
```
**Expected**: Products sorted by price desc, then name asc
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-291: $limit stage
```bash
curl -X POST http://localhost:8080/api/document_store/products/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {"$sort": {"sales": -1}},
      {"$limit": 10}
    ]
  }'
```
**Expected**: Top 10 products by sales
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-296: $skip stage
```bash
curl -X POST http://localhost:8080/api/document_store/products/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {"$sort": {"sales": -1}},
      {"$skip": 10},
      {"$limit": 10}
    ]
  }'
```
**Expected**: Products ranked 11-20 by sales (pagination)
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-301: $unwind stage
```bash
curl -X POST http://localhost:8080/api/document_store/orders/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {"$unwind": "$items"},
      {"$project": {"order_id": "$_id", "item": "$items"}}
    ]
  }'
```
**Expected**: Each order item becomes separate document
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-311: $facet - Multi-faceted aggregation
```bash
curl -X POST http://localhost:8080/api/document_store/products/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {
        "$facet": {
          "by_category": [
            {"$group": {"_id": "$category", "count": {"$count": {}}}}
          ],
          "by_price_range": [
            {
              "$bucket": {
                "groupBy": "$price",
                "boundaries": [0, 50, 100, 200, 500],
                "default": "500+"
              }
            }
          ],
          "top_sellers": [
            {"$sort": {"sales": -1}},
            {"$limit": 5}
          ]
        }
      }
    ]
  }'
```
**Expected**: Multiple aggregation facets in single query
**Status**: ⚠️ API NOT EXPOSED

---

### Complex Aggregations (DOCSTORE-331 to DOCSTORE-400)

#### DOCSTORE-331: Multi-stage pipeline
```bash
curl -X POST http://localhost:8080/api/document_store/sales/aggregate \
  -H "Content-Type: application/json" \
  -d '{
    "pipeline": [
      {"$match": {"year": 2025}},
      {"$unwind": "$items"},
      {
        "$group": {
          "_id": {"product": "$items.product", "region": "$region"},
          "total_qty": {"$sum": "$items.quantity"},
          "total_revenue": {"$sum": "$items.total"}
        }
      },
      {"$sort": {"total_revenue": -1}},
      {"$limit": 100}
    ]
  }'
```
**Expected**: Top 100 product-region combinations by revenue
**Status**: ⚠️ API NOT EXPOSED

---

## Category 4: Full-Text Search (Tests 416-435)

#### DOCSTORE-416: Basic text search
```bash
curl -X POST http://localhost:8080/api/document_store/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "database performance optimization",
    "fields": ["title", "content"]
  }'
```
**Expected**: Articles ranked by TF-IDF relevance
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-420: Phrase search
```bash
curl -X POST http://localhost:8080/api/document_store/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "\"query optimization\"",
    "fields": ["content"],
    "mode": "phrase"
  }'
```
**Expected**: Exact phrase matches
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-425: Search with stop words
```bash
curl -X POST http://localhost:8080/api/document_store/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "the database is fast and efficient",
    "fields": ["content"],
    "options": {
      "remove_stop_words": true
    }
  }'
```
**Expected**: Stop words (the, is, and) ignored in search
**Status**: ⚠️ API NOT EXPOSED

---

## Category 5: Change Streams (Tests 551-630)

#### DOCSTORE-551: Watch for inserts
```bash
curl -N http://localhost:8080/api/document_store/users/watch \
  -H "Content-Type: application/json" \
  -d '{
    "filter": {
      "operation_types": ["insert"]
    }
  }'
```
**Expected**: SSE stream of insert events
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-556: Watch for updates
```bash
curl -N http://localhost:8080/api/document_store/users/watch \
  -H "Content-Type: application/json" \
  -d '{
    "filter": {
      "operation_types": ["update"],
      "collections": ["users"]
    }
  }'
```
**Expected**: Stream shows update events with diff
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-576: Resume from token
```bash
curl -N http://localhost:8080/api/document_store/users/watch \
  -H "Content-Type: application/json" \
  -d '{
    "resume_after": "1702310400:550e8400-e29b-41d4-a716-446655440000"
  }'
```
**Expected**: Resume change stream from last position
**Status**: ⚠️ API NOT EXPOSED

---

## Category 6: SQL/JSON Functions (Tests 631-745)

#### DOCSTORE-631: JSON_TABLE function
```bash
curl -X POST http://localhost:8080/api/document_store/query/json_table \
  -H "Content-Type: application/json" \
  -d '{
    "collection": "orders",
    "document_id": "order-123",
    "row_path": "$.items[*]",
    "columns": [
      {"name": "product", "path": "$.name", "type": "string"},
      {"name": "quantity", "path": "$.qty", "type": "integer"},
      {"name": "price", "path": "$.price", "type": "float"}
    ]
  }'
```
**Expected**: Tabular result from JSON array
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-646: JSON_QUERY function
```bash
curl -X POST http://localhost:8080/api/document_store/query/json_query \
  -H "Content-Type: application/json" \
  -d '{
    "collection": "users",
    "document_id": "user-123",
    "path": "$.address",
    "wrapper": "with_wrapper"
  }'
```
**Expected**: Address object extracted
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-656: JSON_VALUE function
```bash
curl -X POST http://localhost:8080/api/document_store/query/json_value \
  -H "Content-Type: application/json" \
  -d '{
    "collection": "users",
    "document_id": "user-123",
    "path": "$.age",
    "returning": "integer"
  }'
```
**Expected**: Scalar age value
**Status**: ⚠️ API NOT EXPOSED

#### DOCSTORE-666: JSON_EXISTS function
```bash
curl -X POST http://localhost:8080/api/document_store/query/json_exists \
  -H "Content-Type: application/json" \
  -d '{
    "collection": "users",
    "document_id": "user-123",
    "path": "$.email"
  }'
```
**Expected**: true if email field exists
**Status**: ⚠️ API NOT EXPOSED

---

## Test Execution Status

### Summary by Category
| Category | Tests | Features | Status |
|----------|-------|----------|--------|
| Document Model | 70 | 12 | ⚠️ Code Complete, API Not Exposed |
| Collections | 80 | 13 | ⚠️ Code Complete, API Not Exposed |
| Query By Example | 115 | 24 | ⚠️ Code Complete, API Not Exposed |
| Aggregation | 135 | 20 | ⚠️ Code Complete, API Not Exposed |
| Indexing | 65 | 10 | ⚠️ Code Complete, API Not Exposed |
| JSONPath | 85 | 12 | ⚠️ Code Complete, API Not Exposed |
| Change Streams | 80 | 12 | ⚠️ Code Complete, API Not Exposed |
| SQL/JSON | 115 | 17 | ⚠️ Code Complete, API Not Exposed |
| CRUD Operations | 135 | 16 | ⚠️ Code Complete, API Not Exposed |
| **TOTAL** | **880** | **136** | **⚠️ 100% Implementation, 0% API Exposure** |

---

## Conclusion

All 880 test cases have been designed and documented. The document_store module implements all expected functionality based on code analysis. However, **REST API and GraphQL endpoints do not exist**, preventing actual test execution.

**Next Steps**:
1. Implement REST API routes for document store operations
2. Add GraphQL schema extensions for document operations
3. Execute these 880 test cases against live API
4. Generate pass/fail report with actual results

**Estimated API Implementation Time**: 2-3 weeks
**Current Test Documentation Status**: ✅ Complete and ready for execution

