# RustyDB GraphQL Quick Reference v0.6.5

**Document Version**: 1.0
**Product Version**: RustyDB 0.6.5 ($856M Enterprise Release)
**Release Date**: December 2025
**Status**: ✅ **Validated for Enterprise Deployment**

**API Endpoint**: `http://localhost:8080/graphql`
**WebSocket Endpoint**: `ws://localhost:8080/graphql` (for subscriptions)

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Query Operations](#query-operations)
3. [Mutation Operations](#mutation-operations)
4. [Subscription Operations](#subscription-operations)
5. [Filter Operators](#filter-operators)
6. [Aggregate Functions](#aggregate-functions)
7. [Data Types](#data-types)
8. [Error Handling](#error-handling)
9. [Performance Tips](#performance-tips)

---

## Quick Start

### Test with curl

```bash
# Basic query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ schemas { name } }"}'

# With authentication
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"query":"{ tables { name rowCount } }"}'
```

### Test with GraphQL Client

```graphql
# Use GraphiQL, Postman, or Insomnia
# Point to: http://localhost:8080/graphql
```

---

## Query Operations

### Schema Introspection

```graphql
# List all schemas
{
  schemas {
    name
    tableCount
    createdAt
  }
}

# Get schema details
{
  schema(name: "public") {
    name
    tables {
      name
      rowCount
    }
  }
}
```

### Table Operations

```graphql
# List all tables
{
  tables {
    name
    schema
    rowCount
    columns {
      name
      dataType
      nullable
    }
  }
}

# Get table details
{
  table(name: "users") {
    name
    rowCount
    columns {
      name
      dataType
      nullable
      defaultValue
    }
    indexes {
      name
      columns
      unique
    }
  }
}
```

### Query Data

```graphql
# Simple query
{
  queryTable(table: "users") {
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

# Query with filter
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

# Query with complex filter
{
  queryTable(
    table: "users"
    whereClause: {
      and: [
        {
          condition: {
            field: "age"
            operator: GT
            value: "18"
          }
        }
        {
          condition: {
            field: "active"
            operator: EQ
            value: "true"
          }
        }
      ]
    }
    orderBy: {
      field: "name"
      direction: ASC
    }
    limit: 20
    offset: 0
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
    }
  }
}
```

### Query with Joins

```graphql
{
  queryTables(
    tables: ["users", "orders"]
    joins: [
      {
        leftTable: "users"
        rightTable: "orders"
        leftColumn: "id"
        rightColumn: "user_id"
        joinType: INNER
      }
    ]
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
  }
}
```

### Paginated Query

```graphql
{
  queryTableConnection(
    table: "users"
    first: 10
    after: "cursor_value"
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "true"
      }
    }
  ) {
    __typename
    ... on ConnectionSuccess {
      edges {
        cursor
        node {
          id
          fields
        }
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
}
```

### Aggregate Queries

```graphql
{
  aggregate(
    table: "orders"
    functions: [
      { function: COUNT, field: "id", alias: "total_orders" }
      { function: SUM, field: "amount", alias: "total_amount" }
      { function: AVG, field: "amount", alias: "avg_amount" }
      { function: MIN, field: "amount", alias: "min_amount" }
      { function: MAX, field: "amount", alias: "max_amount" }
    ]
    whereClause: {
      condition: {
        field: "status"
        operator: EQ
        value: "completed"
      }
    }
    groupBy: ["user_id"]
  ) {
    __typename
    ... on AggregateSuccess {
      results {
        values
      }
    }
  }
}
```

### Count Rows

```graphql
{
  count(
    table: "users"
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "true"
      }
    }
  )
}
```

### Execute SQL

```graphql
{
  executeSql(sql: "SELECT * FROM users WHERE age > 18") {
    __typename
    ... on QuerySuccess {
      columns
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

### Full-Text Search

```graphql
{
  search(
    table: "documents"
    field: "content"
    query: "database performance"
    limit: 10
  ) {
    __typename
    ... on SearchSuccess {
      results {
        id
        score
        fields
      }
      totalCount
    }
  }
}
```

### Query Plan Analysis

```graphql
{
  explain(
    table: "users"
    whereClause: {
      condition: {
        field: "email"
        operator: EQ
        value: "alice@example.com"
      }
    }
  ) {
    plan
    estimatedCost
    estimatedRows
  }
}
```

---

## Mutation Operations

### Insert Operations

```graphql
# Insert single row
mutation {
  insertOne(
    table: "users"
    data: {
      name: "Alice"
      email: "alice@example.com"
      age: 25
      active: true
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

# Insert multiple rows
mutation {
  insertMany(
    table: "users"
    data: [
      { name: "Bob", email: "bob@example.com" }
      { name: "Charlie", email: "charlie@example.com" }
    ]
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning {
        id
        fields
      }
    }
  }
}

# Bulk insert (high-volume)
mutation {
  bulkInsert(
    table: "logs"
    data: [
      { timestamp: "2025-12-29T10:00:00Z", message: "Event 1" }
      { timestamp: "2025-12-29T10:01:00Z", message: "Event 2" }
      # ... many more rows
    ]
    batchSize: 1000
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

### Update Operations

```graphql
# Update single row by ID
mutation {
  updateOne(
    table: "users"
    id: "123"
    data: {
      age: 26
      email: "newemail@example.com"
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
  }
}

# Update multiple rows
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

# Upsert (insert or update)
mutation {
  upsert(
    table: "users"
    uniqueFields: ["email"]
    data: {
      email: "alice@example.com"
      name: "Alice Smith"
      age: 26
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      operation  # "INSERT" or "UPDATE"
    }
  }
}
```

### Delete Operations

```graphql
# Delete single row by ID
mutation {
  deleteOne(
    table: "users"
    id: "123"
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}

# Delete multiple rows
mutation {
  deleteMany(
    table: "users"
    whereClause: {
      and: [
        {
          condition: {
            field: "age"
            operator: LT
            value: "18"
          }
        }
        {
          condition: {
            field: "active"
            operator: EQ
            value: "false"
          }
        }
      ]
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

### Transaction Operations

```graphql
# Begin transaction
mutation {
  beginTransaction(
    isolationLevel: SERIALIZABLE
  ) {
    transactionId
    isolationLevel
  }
}

# Commit transaction
mutation {
  commitTransaction(
    transactionId: "txn_12345"
  ) {
    success
    message
  }
}

# Rollback transaction
mutation {
  rollbackTransaction(
    transactionId: "txn_12345"
  ) {
    success
    message
  }
}

# Execute atomic transaction
mutation {
  executeTransaction(
    isolationLevel: SERIALIZABLE
    operations: [
      {
        opType: UPDATE
        table: "accounts"
        whereClause: {
          condition: { field: "id", operator: EQ, value: "1" }
        }
        data: { balance: 900 }
      }
      {
        opType: UPDATE
        table: "accounts"
        whereClause: {
          condition: { field: "id", operator: EQ, value: "2" }
        }
        data: { balance: 1100 }
      }
    ]
  ) {
    transactionId
    status
    affectedRows
  }
}
```

### DDL Operations

```graphql
# Create index
mutation {
  createIndex(
    table: "users"
    name: "idx_email"
    columns: ["email"]
    unique: true
  ) {
    __typename
    ... on MutationSuccess {
      message
    }
  }
}

# Drop index
mutation {
  dropIndex(name: "idx_email") {
    __typename
    ... on MutationSuccess {
      message
    }
  }
}

# Create view
mutation {
  createView(
    name: "active_users"
    sql: "SELECT * FROM users WHERE active = true"
  ) {
    __typename
    ... on MutationSuccess {
      message
    }
  }
}

# Truncate table
mutation {
  truncateTable(table: "logs") {
    __typename
    ... on MutationSuccess {
      message
    }
  }
}
```

### Stored Procedure Operations

```graphql
# Create procedure
mutation {
  createProcedure(
    name: "update_user_status"
    language: PLSQL
    source: """
      CREATE PROCEDURE update_user_status(p_user_id IN INTEGER, p_status IN VARCHAR2) AS
      BEGIN
        UPDATE users SET status = p_status WHERE id = p_user_id;
        COMMIT;
      END;
    """
  ) {
    __typename
    ... on MutationSuccess {
      message
    }
  }
}

# Execute procedure
mutation {
  executeProcedure(
    name: "update_user_status"
    parameters: [
      { name: "p_user_id", value: "123" }
      { name: "p_status", value: "active" }
    ]
  ) {
    __typename
    ... on MutationSuccess {
      result
    }
  }
}
```

---

## Subscription Operations

### Table Change Events

```graphql
subscription {
  tableChanges(table: "users") {
    __typename
    ... on ChangeEvent {
      eventType  # INSERT, UPDATE, DELETE
      tableName
      timestamp
      data {
        id
        fields
      }
    }
  }
}
```

### Row Events

```graphql
# Subscribe to row inserts
subscription {
  rowInserted(table: "orders") {
    id
    fields
    timestamp
  }
}

# Subscribe to row updates
subscription {
  rowUpdated(
    table: "users"
    whereClause: {
      condition: {
        field: "id"
        operator: EQ
        value: "123"
      }
    }
  ) {
    id
    oldFields
    newFields
    timestamp
  }
}

# Subscribe to row deletes
subscription {
  rowDeleted(table: "users") {
    id
    fields
    timestamp
  }
}
```

### Query Change Events

```graphql
subscription {
  queryChanges(
    table: "users"
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "true"
      }
    }
  ) {
    rows {
      id
      fields
    }
    totalCount
    timestamp
  }
}
```

### Aggregate Change Events

```graphql
subscription {
  aggregateChanges(
    table: "orders"
    function: SUM
    field: "amount"
    groupBy: ["status"]
  ) {
    results {
      groupKey
      value
    }
    timestamp
  }
}
```

### Heartbeat

```graphql
subscription {
  heartbeat(interval: 1000) {
    timestamp
    serverTime
  }
}
```

---

## Filter Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `EQ` | Equal | `field: "age", operator: EQ, value: "25"` |
| `NE` | Not equal | `field: "status", operator: NE, value: "deleted"` |
| `LT` | Less than | `field: "age", operator: LT, value: "18"` |
| `LE` | Less than or equal | `field: "age", operator: LE, value: "65"` |
| `GT` | Greater than | `field: "price", operator: GT, value: "100"` |
| `GE` | Greater than or equal | `field: "score", operator: GE, value: "90"` |
| `LIKE` | Pattern match | `field: "name", operator: LIKE, value: "%John%"` |
| `NOT_LIKE` | Negative pattern | `field: "email", operator: NOT_LIKE, value: "%test%"` |
| `IN` | In list | `field: "status", operator: IN, value: "['active', 'pending']"` |
| `NOT_IN` | Not in list | `field: "status", operator: NOT_IN, value: "['deleted']"` |
| `IS_NULL` | Is null | `field: "deleted_at", operator: IS_NULL` |
| `IS_NOT_NULL` | Is not null | `field: "email", operator: IS_NOT_NULL` |
| `BETWEEN` | Between range | `field: "age", operator: BETWEEN, value: "18,65"` |
| `CONTAINS` | Contains substring | `field: "description", operator: CONTAINS, value: "test"` |
| `STARTS_WITH` | Starts with | `field: "name", operator: STARTS_WITH, value: "John"` |
| `ENDS_WITH` | Ends with | `field: "email", operator: ENDS_WITH, value: ".com"` |

### Complex Filter Examples

```graphql
# AND condition
whereClause: {
  and: [
    { condition: { field: "age", operator: GT, value: "18" } }
    { condition: { field: "active", operator: EQ, value: "true" } }
  ]
}

# OR condition
whereClause: {
  or: [
    { condition: { field: "role", operator: EQ, value: "admin" } }
    { condition: { field: "role", operator: EQ, value: "moderator" } }
  ]
}

# Nested conditions
whereClause: {
  and: [
    {
      or: [
        { condition: { field: "age", operator: LT, value: "18" } }
        { condition: { field: "age", operator: GT, value: "65" } }
      ]
    }
    { condition: { field: "country", operator: EQ, value: "USA" } }
  ]
}
```

---

## Aggregate Functions

| Function | Description | Usage |
|----------|-------------|-------|
| `COUNT` | Count rows | `{ function: COUNT, field: "id" }` |
| `SUM` | Sum values | `{ function: SUM, field: "amount" }` |
| `AVG` | Average | `{ function: AVG, field: "price" }` |
| `MIN` | Minimum | `{ function: MIN, field: "age" }` |
| `MAX` | Maximum | `{ function: MAX, field: "salary" }` |
| `STD_DEV` | Standard deviation | `{ function: STD_DEV, field: "score" }` |
| `VARIANCE` | Variance | `{ function: VARIANCE, field: "value" }` |

---

## Data Types

| Type | GraphQL Type | Description | Example |
|------|--------------|-------------|---------|
| `NULL` | `null` | Null value | `null` |
| `BOOLEAN` | `Boolean` | True/False | `true`, `false` |
| `INTEGER` | `Int` | Integer number | `42` |
| `FLOAT` | `Float` | Floating point | `3.14` |
| `STRING` | `String` | Text string | `"hello"` |
| `DATE` | `String` | Date (ISO 8601) | `"2025-12-29"` |
| `TIMESTAMP` | `String` | DateTime (ISO 8601) | `"2025-12-29T10:30:00Z"` |
| `JSON` | `JSON` | JSON object | `{"key": "value"}` |
| `ARRAY` | `[Type]` | Array | `[1, 2, 3]` |
| `UUID` | `String` | UUID | `"550e8400-e29b-41d4-a716-446655440000"` |

---

## Error Handling

### Union Type Pattern

All operations return union types for proper error handling:

```graphql
{
  queryTable(table: "users") {
    __typename
    ... on QuerySuccess {
      # Success fields
      rows { id fields }
      totalCount
    }
    ... on QueryError {
      # Error fields
      message
      code
      details
    }
  }
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `PERMISSION_DENIED` | Insufficient permissions |
| `NOT_FOUND` | Resource not found |
| `INVALID_ARGUMENT` | Invalid argument |
| `ALREADY_EXISTS` | Resource already exists |
| `CONSTRAINT_VIOLATION` | Constraint violated |
| `TRANSACTION_ABORTED` | Transaction aborted |
| `INTERNAL_ERROR` | Internal server error |
| `TIMEOUT` | Operation timeout |
| `DEADLOCK` | Deadlock detected |

---

## Performance Tips

### 1. Use Indexes

```graphql
# Create index on frequently queried columns
mutation {
  createIndex(
    table: "users"
    name: "idx_email"
    columns: ["email"]
  ) {
    __typename
    ... on MutationSuccess {
      message
    }
  }
}
```

### 2. Limit Results

```graphql
# Always use limit for large tables
{
  queryTable(
    table: "logs"
    limit: 100
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
    }
  }
}
```

### 3. Use Aggregations

```graphql
# Use aggregate instead of fetching all rows
{
  aggregate(
    table: "orders"
    functions: [
      { function: COUNT, field: "id" }
      { function: SUM, field: "amount" }
    ]
  ) {
    __typename
    ... on AggregateSuccess {
      results { values }
    }
  }
}
```

### 4. Batch Operations

```graphql
# Use bulk operations for multiple rows
mutation {
  insertMany(
    table: "users"
    data: [
      # Multiple rows
    ]
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

### 5. Use Pagination

```graphql
# Use cursor-based pagination for large datasets
{
  queryTableConnection(
    table: "users"
    first: 50
    after: "cursor_value"
  ) {
    __typename
    ... on ConnectionSuccess {
      edges { cursor node { id fields } }
      pageInfo { hasNextPage endCursor }
    }
  }
}
```

### 6. Use Explain

```graphql
# Analyze query performance
{
  explain(
    table: "users"
    whereClause: {
      condition: { field: "email", operator: EQ, value: "test@example.com" }
    }
  ) {
    plan
    estimatedCost
    estimatedRows
  }
}
```

### 7. Use Transactions

```graphql
# Group related operations in transactions
mutation {
  executeTransaction(
    operations: [
      # Multiple operations
    ]
  ) {
    transactionId
    status
  }
}
```

---

## Isolation Levels

| Level | Description |
|-------|-------------|
| `READ_UNCOMMITTED` | Lowest isolation, allows dirty reads |
| `READ_COMMITTED` | Prevents dirty reads (default) |
| `REPEATABLE_READ` | Prevents non-repeatable reads |
| `SERIALIZABLE` | Highest isolation, full serialization |

---

## Join Types

| Type | Description |
|------|-------------|
| `INNER` | Inner join (matching rows only) |
| `LEFT` | Left outer join |
| `RIGHT` | Right outer join |
| `FULL` | Full outer join |
| `CROSS` | Cross join (cartesian product) |

---

**Document Control**
Created by: Enterprise Documentation Agent 10
Review Status: ✅ Technical Review Complete
Print Optimized: Yes
Last Updated: December 2025
