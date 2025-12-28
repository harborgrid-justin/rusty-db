# RustyDB v0.6.0 - GraphQL Quick Reference

**Endpoint**: `http://localhost:8080/graphql`
**Method**: POST
**Content-Type**: application/json
**Version**: 0.6.0 | **Updated**: December 28, 2025

---

## Quick Test

```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ schemas { name } }"}'
```

---

## Core Operations

### Queries (14 operations)

| Operation | Purpose | Example |
|-----------|---------|---------|
| `schemas` | List schemas | `{ schemas { name } }` |
| `tables` | List tables | `{ tables { name rowCount } }` |
| `table(name)` | Get table | `{ table(name: "users") { columns { name type } } }` |
| `queryTable` | Query data | See below |
| `count` | Count rows | `{ count(table: "users") }` |
| `aggregate` | Aggregations | See below |
| `executeSql` | Raw SQL | `{ executeSql(sql: "SELECT 1") }` (admin only) |
| `search` | Full-text | `{ search(table: "docs", query: "database") }` |
| `explain` | Query plan | `{ explain(sql: "SELECT * FROM users") }` |

### Mutations (30 operations)

| Operation | Purpose | Example |
|-----------|---------|---------|
| `insertOne` | Insert row | See below |
| `insertMany` | Insert multiple | See below |
| `updateOne` | Update by ID | See below |
| `updateMany` | Bulk update | See below |
| `deleteOne` | Delete by ID | See below |
| `deleteMany` | Bulk delete | See below |
| `beginTransaction` | Start txn | `beginTransaction { transactionId }` |
| `commitTransaction` | Commit txn | `commitTransaction(id: "...") { status }` |
| `rollbackTransaction` | Rollback txn | `rollbackTransaction(id: "...") { status }` |

---

## Common Patterns

### List All Tables
```graphql
{
  tables {
    name
    rowCount
    columns {
      name
      type
    }
  }
}
```

**curl**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ tables { name rowCount } }"}'
```

---

### Query Table
```graphql
{
  queryTable(
    table: "users"
    limit: 10
  ) {
    __typename
    ... on QuerySuccess {
      rows {
        id
        fields
      }
      totalCount
    }
    ... on QueryError {
      message
      code
    }
  }
}
```

**curl**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ queryTable(table: \"users\", limit: 10) { __typename ... on QuerySuccess { rows { id fields } totalCount } ... on QueryError { message code } } }"}'
```

---

### Query with Filter
```graphql
{
  queryTable(
    table: "users"
    whereClause: {
      condition: {
        field: "age"
        operator: GT
        value: "25"
      }
    }
    limit: 10
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
    }
  }
}
```

**curl**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ queryTable(table: \"users\", whereClause: { condition: { field: \"age\", operator: GT, value: \"25\" } }, limit: 10) { __typename ... on QuerySuccess { rows { id fields } } } }"}'
```

---

### Insert Single Row
```graphql
mutation {
  insertOne(
    table: "users"
    data: {
      name: "Alice"
      email: "alice@example.com"
      age: 30
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning {
        id
        fields
      }
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

**curl**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { insertOne(table: \"users\", data: { name: \"Alice\", email: \"alice@example.com\", age: 30 }) { __typename ... on MutationSuccess { affectedRows } ... on MutationError { message } } }"}'
```

---

### Insert Multiple Rows
```graphql
mutation {
  insertMany(
    table: "users"
    data: [
      { name: "Alice", email: "alice@example.com" }
      { name: "Bob", email: "bob@example.com" }
      { name: "Charlie", email: "charlie@example.com" }
    ]
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

---

### Update Single Row
```graphql
mutation {
  updateOne(
    table: "users"
    id: "1"
    data: {
      email: "newemail@example.com"
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning { id fields }
    }
  }
}
```

---

### Update Multiple Rows
```graphql
mutation {
  updateMany(
    table: "users"
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "false"
      }
    }
    data: {
      status: "archived"
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

---

### Delete Row
```graphql
mutation {
  deleteOne(
    table: "users"
    id: "1"
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

---

### Bulk Delete
```graphql
mutation {
  deleteMany(
    table: "users"
    whereClause: {
      condition: {
        field: "age"
        operator: LT
        value: "18"
      }
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

---

### Count Rows
```graphql
{
  count(table: "users")
}
```

**curl**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ count(table: \"users\") }"}'
```

---

### Aggregate Query
```graphql
{
  aggregate(
    table: "products"
    operations: [
      { function: AVG, field: "price" }
      { function: SUM, field: "quantity" }
      { function: MIN, field: "price" }
      { function: MAX, field: "price" }
    ]
  ) {
    __typename
    ... on AggregateSuccess {
      results {
        function
        field
        value
      }
    }
  }
}
```

---

### Transaction
```graphql
mutation {
  executeTransaction(
    isolationLevel: SERIALIZABLE
    operations: [
      {
        opType: INSERT
        table: "accounts"
        data: { id: 1, balance: 1000 }
      }
      {
        opType: UPDATE
        table: "accounts"
        whereClause: {
          condition: {
            field: "id"
            operator: EQ
            value: "2"
          }
        }
        data: { balance: 500 }
      }
    ]
  ) {
    transactionId
    status
  }
}
```

---

## Filter Operators

| Operator | SQL | Example |
|----------|-----|---------|
| `EQ` | `=` | `{ field: "age", operator: EQ, value: "25" }` |
| `NE` | `!=` | `{ field: "age", operator: NE, value: "25" }` |
| `LT` | `<` | `{ field: "age", operator: LT, value: "25" }` |
| `LE` | `<=` | `{ field: "age", operator: LE, value: "25" }` |
| `GT` | `>` | `{ field: "age", operator: GT, value: "25" }` |
| `GE` | `>=` | `{ field: "age", operator: GE, value: "25" }` |
| `LIKE` | `LIKE` | `{ field: "name", operator: LIKE, value: "%Alice%" }` |
| `NOT_LIKE` | `NOT LIKE` | `{ field: "name", operator: NOT_LIKE, value: "%test%" }` |
| `IN` | `IN` | `{ field: "status", operator: IN, values: ["active", "pending"] }` |
| `NOT_IN` | `NOT IN` | `{ field: "status", operator: NOT_IN, values: ["deleted"] }` |
| `IS_NULL` | `IS NULL` | `{ field: "email", operator: IS_NULL }` |
| `IS_NOT_NULL` | `IS NOT NULL` | `{ field: "email", operator: IS_NOT_NULL }` |
| `BETWEEN` | `BETWEEN` | `{ field: "age", operator: BETWEEN, min: "18", max: "65" }` |

---

## Aggregate Functions

| Function | Description |
|----------|-------------|
| `COUNT` | Count rows |
| `SUM` | Sum values |
| `AVG` | Average value |
| `MIN` | Minimum value |
| `MAX` | Maximum value |
| `STD_DEV` | Standard deviation |
| `VARIANCE` | Variance |

---

## Data Types

| Type | Example |
|------|---------|
| `NULL` | `null` |
| `BOOLEAN` | `true`, `false` |
| `INTEGER` | `42` |
| `FLOAT` | `3.14` |
| `STRING` | `"hello"` |
| `DATE` | `"2025-12-28"` |
| `TIMESTAMP` | `"2025-12-28T10:30:00Z"` |
| `JSON` | `{"key": "value"}` |
| `ARRAY` | `[1, 2, 3]` |

---

## Isolation Levels

- `READ_UNCOMMITTED` - Lowest isolation
- `READ_COMMITTED` - Default, prevents dirty reads
- `REPEATABLE_READ` - Prevents non-repeatable reads
- `SERIALIZABLE` - Highest isolation

---

## Error Handling

Always use union types for proper error handling:

```graphql
{
  __typename
  ... on QuerySuccess {
    # Success fields
  }
  ... on QueryError {
    message
    code
    details
  }
}
```

### Common Error Codes

- `PERMISSION_DENIED` - Insufficient permissions
- `NOT_FOUND` - Resource not found
- `INVALID_ARGUMENT` - Invalid argument
- `ALREADY_EXISTS` - Resource exists
- `CONSTRAINT_VIOLATION` - Constraint violated
- `TRANSACTION_ABORTED` - Transaction aborted
- `INTERNAL_ERROR` - Internal error

---

## Pagination

### Cursor-Based
```graphql
{
  queryTableConnection(
    table: "users"
    first: 10
    after: "cursor_value"
  ) {
    edges {
      node { id fields }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

### Offset-Based
```graphql
{
  queryTable(
    table: "users"
    limit: 10
    offset: 20
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
    }
  }
}
```

---

## Complex Filters

### AND Conditions
```graphql
whereClause: {
  and: [
    { condition: { field: "age", operator: GT, value: "18" } }
    { condition: { field: "active", operator: EQ, value: "true" } }
  ]
}
```

### OR Conditions
```graphql
whereClause: {
  or: [
    { condition: { field: "age", operator: LT, value: "18" } }
    { condition: { field: "age", operator: GT, value: "65" } }
  ]
}
```

### Nested Conditions
```graphql
whereClause: {
  and: [
    {
      or: [
        { condition: { field: "status", operator: EQ, value: "active" } }
        { condition: { field: "status", operator: EQ, value: "pending" } }
      ]
    }
    { condition: { field: "age", operator: GT, value: "18" } }
  ]
}
```

---

## Performance Tips

1. **Always use `limit`** for large tables
2. **Use `count()`** instead of fetching all rows
3. **Use `aggregate()`** for calculations
4. **Use indexed columns** in WHERE clauses
5. **Batch operations** with `insertMany`, `updateMany`, `deleteMany`
6. **Use `explain`** to analyze query plans
7. **Use pagination** for large result sets
8. **Use subscriptions** for real-time updates

---

## Schema Introspection

### List All Types
```graphql
{
  __schema {
    types {
      name
      kind
    }
  }
}
```

### Query Fields
```graphql
{
  __type(name: "QueryRoot") {
    fields {
      name
      description
      type {
        name
      }
    }
  }
}
```

### Mutation Fields
```graphql
{
  __type(name: "MutationRoot") {
    fields {
      name
      description
    }
  }
}
```

---

## Subscriptions (WebSocket)

**Connect**: `ws://localhost:8080/ws`

### Table Changes
```graphql
subscription {
  tableChanges(table: "users") {
    operation
    table
    rowId
    data
    timestamp
  }
}
```

### Row Inserted
```graphql
subscription {
  rowInserted(table: "users") {
    id
    fields
  }
}
```

### Row Updated
```graphql
subscription {
  rowUpdated(table: "users", rowId: "1") {
    id
    fields
  }
}
```

### Row Deleted
```graphql
subscription {
  rowDeleted(table: "users") {
    id
  }
}
```

---

## Complete Examples

### User Management
```bash
# Create users table
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { executeSql(sql: \"CREATE TABLE users (id INTEGER, name TEXT, email TEXT, age INTEGER, active BOOLEAN)\") { __typename } }"}'

# Insert user
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { insertOne(table: \"users\", data: { id: 1, name: \"Alice\", email: \"alice@example.com\", age: 30, active: true }) { __typename ... on MutationSuccess { affectedRows } } }"}'

# Query users
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ queryTable(table: \"users\", limit: 10) { __typename ... on QuerySuccess { rows { id fields } } } }"}'

# Update user
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { updateOne(table: \"users\", id: \"1\", data: { email: \"newemail@example.com\" }) { __typename ... on MutationSuccess { affectedRows } } }"}'

# Delete user
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { deleteOne(table: \"users\", id: \"1\") { __typename ... on MutationSuccess { affectedRows } } }"}'
```

---

## Testing

### Run Test Suite
```bash
cd /home/user/rusty-db
./graphql_curl_commands.sh
```

### Test Specific Operation
```bash
# Test queries
./graphql_curl_commands.sh queries

# Test mutations
./graphql_curl_commands.sh mutations

# Test schema introspection
./graphql_curl_commands.sh schema
```

---

## Documentation

- **graphql_test_results.md** - Detailed test results (90 tests)
- **graphql_examples.md** - Complete examples (56 examples)
- **graphql_curl_commands.sh** - Executable test script

---

**GraphQL Reference** | RustyDB v0.6.0 | Enterprise Database Server
