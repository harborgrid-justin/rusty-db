# RustyDB GraphQL API Reference

**RustyDB v0.6.0 - Enterprise Server**
**GraphQL Version**: June 2018 Standard
**Last Updated**: 2025-12-28
**Endpoint**: `http://localhost:8080/graphql`

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Schema Overview](#schema-overview)
4. [Query Operations](#query-operations)
5. [Mutation Operations](#mutation-operations)
6. [Subscription Operations](#subscription-operations)
7. [Transaction Support](#transaction-support)
8. [Type System](#type-system)
9. [Error Handling](#error-handling)
10. [Performance & Best Practices](#performance--best-practices)

---

## Introduction

RustyDB provides a comprehensive GraphQL API for flexible, type-safe database operations. The GraphQL interface offers:

- **Flexible Queries**: Request exactly the data you need
- **Strong Typing**: Type-safe operations with schema validation
- **Real-time Updates**: WebSocket-based subscriptions
- **Transaction Support**: Full ACID transaction control
- **Batching**: Multiple operations in a single request
- **Introspection**: Self-documenting API

### API Status

**Coverage**:
- **Queries**: 14 operations (100% complete)
- **Mutations**: 30 operations (100% complete)
- **Subscriptions**: 12 active (41%), 16 planned (55%)
- **Test Pass Rate**: 69.3% (101 tests, 70 passing)

### Endpoints

**HTTP Endpoint**: `POST http://localhost:8080/graphql`
**WebSocket Endpoint**: `ws://localhost:8080/graphql/ws` (Subscriptions)
**Playground**: `http://localhost:8080/graphql` (GET request)

---

## Getting Started

### Quick Start with cURL

```bash
# Simple query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ schemas { name } }"}'

# Query with variables
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetUser($id: ID!) { user(id: $id) { name email } }",
    "variables": {"id": "123"}
  }'
```

### Authentication

GraphQL API supports JWT token authentication (optional in development):

```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{"query":"{ tables { name } }"}'
```

### GraphQL Playground

Access the interactive GraphQL Playground:
```
http://localhost:8080/graphql
```

The playground provides:
- Auto-complete and syntax highlighting
- Schema documentation browser
- Query history
- Variable editor

---

## Schema Overview

### Root Types

```graphql
type Query {
  # Schema operations
  schemas: [Schema!]!
  schema(name: String!): Schema
  tables(schema: String): [Table!]!
  table(name: String!): Table

  # Data queries
  queryTable(
    table: String!
    whereClause: WhereClause
    orderBy: [OrderBy!]
    limit: Int
    offset: Int
  ): QueryResult!

  # Aggregations
  aggregate(
    table: String!
    aggregations: [Aggregation!]!
    groupBy: [String!]
    whereClause: WhereClause
  ): AggregateResult!

  # Utilities
  explain(sql: String!): QueryPlan!
  search(
    table: String!
    searchText: String!
    fields: [String!]
  ): SearchResult!
}

type Mutation {
  # Data manipulation
  insertOne(table: String!, data: JSON!): MutationResult!
  insertMany(table: String!, data: [JSON!]!): MutationResult!
  updateOne(table: String!, id: ID!, data: JSON!): MutationResult!
  updateMany(table: String!, whereClause: WhereClause!, data: JSON!): MutationResult!
  deleteOne(table: String!, id: ID!): MutationResult!
  deleteMany(table: String!, whereClause: WhereClause!): MutationResult!
  upsert(table: String!, data: JSON!, conflictColumns: [String!]): MutationResult!

  # Transactions
  beginTransaction(isolationLevel: IsolationLevel): TransactionResponse!
  commitTransaction(transactionId: String!): TransactionStatusResponse!
  rollbackTransaction(transactionId: String!): TransactionStatusResponse!
  executeTransaction(
    operations: [TransactionOperation!]!
    isolationLevel: IsolationLevel
  ): ExecuteTransactionResponse!

  # DDL operations
  createTable(name: String!, columns: [ColumnDef!]!): DDLResult!
  dropTable(name: String!, cascade: Boolean): DDLResult!
  createIndex(table: String!, name: String!, columns: [String!]!): DDLResult!
}

type Subscription {
  # Table changes
  tableChanges(
    table: String!
    operations: [ChangeType!]
  ): TableChange!

  # Row-level changes
  rowInserted(table: String!): RowChange!
  rowUpdated(table: String!): RowChange!
  rowDeleted(table: String!): RowChange!

  # Aggregates
  aggregateChanges(
    table: String!
    aggregation: Aggregation!
  ): AggregateChange!

  # System
  systemMetrics: MetricsUpdate!
  queryExecution(queryId: ID!): QueryProgress!
  heartbeat: Heartbeat!
}
```

---

## Query Operations

### Schema Queries

#### List Schemas

```graphql
query {
  schemas {
    name
    owner
    tableCount
    totalSize
  }
}
```

**Response**:
```json
{
  "data": {
    "schemas": [
      {
        "name": "public",
        "owner": "postgres",
        "tableCount": 15,
        "totalSize": 1048576
      }
    ]
  }
}
```

#### List Tables

```graphql
query {
  tables(schema: "public") {
    name
    rowCount
    sizeBytes
    columns {
      name
      dataType
      nullable
    }
  }
}
```

### Data Queries

#### Simple Query

```graphql
query {
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

#### Query with Filter

```graphql
query GetActiveUsers {
  queryTable(
    table: "users"
    whereClause: {
      condition: {
        field: "status"
        operator: EQ
        value: "active"
      }
    }
    orderBy: [{ field: "created_at", direction: DESC }]
    limit: 50
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

#### Complex Filter (AND/OR)

```graphql
query GetFilteredUsers {
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
          or: [
            {
              condition: {
                field: "country"
                operator: EQ
                value: "US"
              }
            }
            {
              condition: {
                field: "country"
                operator: EQ
                value: "CA"
              }
            }
          ]
        }
      ]
    }
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
    }
  }
}
```

### Aggregation Queries

```graphql
query GetUserStats {
  aggregate(
    table: "users"
    aggregations: [
      { function: COUNT, field: "id", alias: "total_users" }
      { function: AVG, field: "age", alias: "avg_age" }
      { function: MAX, field: "created_at", alias: "newest_user" }
    ]
    groupBy: ["country"]
  ) {
    __typename
    ... on AggregateSuccess {
      results {
        groupValues
        aggregates
      }
    }
  }
}
```

### Full-Text Search

```graphql
query SearchUsers {
  search(
    table: "users"
    searchText: "john"
    fields: ["name", "email", "bio"]
  ) {
    __typename
    ... on SearchSuccess {
      results {
        id
        score
        highlights
      }
    }
  }
}
```

---

## Mutation Operations

### Insert Operations

#### Insert Single Row

```graphql
mutation CreateUser {
  insertOne(
    table: "users"
    data: {
      name: "John Doe"
      email: "john@example.com"
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

**Response**:
```json
{
  "data": {
    "insertOne": {
      "__typename": "MutationSuccess",
      "affectedRows": 1,
      "returning": [
        {
          "id": "123",
          "fields": {
            "name": "John Doe",
            "email": "john@example.com",
            "age": 30,
            "created_at": "2025-12-28T10:00:00Z"
          }
        }
      ]
    }
  }
}
```

#### Insert Multiple Rows

```graphql
mutation CreateUsers {
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

### Update Operations

#### Update Single Row

```graphql
mutation UpdateUser {
  updateOne(
    table: "users"
    id: "123"
    data: {
      email: "newemail@example.com"
      updatedAt: "2025-12-28T10:00:00Z"
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
```

#### Update Multiple Rows

```graphql
mutation ArchiveInactiveUsers {
  updateMany(
    table: "users"
    whereClause: {
      condition: {
        field: "last_login"
        operator: LT
        value: "2024-01-01T00:00:00Z"
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

### Delete Operations

#### Delete Single Row

```graphql
mutation DeleteUser {
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
```

#### Delete Multiple Rows

```graphql
mutation DeleteArchivedUsers {
  deleteMany(
    table: "users"
    whereClause: {
      condition: {
        field: "status"
        operator: EQ
        value: "archived"
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

### Upsert Operation

```graphql
mutation UpsertUser {
  upsert(
    table: "users"
    data: {
      email: "john@example.com"
      name: "John Doe Updated"
    }
    conflictColumns: ["email"]
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
```

---

## Transaction Support

### Manual Transaction Control

#### Begin Transaction

```graphql
mutation {
  beginTransaction(isolationLevel: SERIALIZABLE) {
    transactionId
    status
    timestamp
    isolationLevel
  }
}
```

**Response**:
```json
{
  "data": {
    "beginTransaction": {
      "transactionId": "88790068-3f05-42fb-a5f8-126ccedff088",
      "status": "ACTIVE",
      "timestamp": "2025-12-28T10:00:00Z",
      "isolationLevel": "SERIALIZABLE"
    }
  }
}
```

#### Commit Transaction

```graphql
mutation {
  commitTransaction(
    transactionId: "88790068-3f05-42fb-a5f8-126ccedff088"
  ) {
    success
    transactionId
    error
  }
}
```

#### Rollback Transaction

```graphql
mutation {
  rollbackTransaction(
    transactionId: "88790068-3f05-42fb-a5f8-126ccedff088"
  ) {
    success
    transactionId
    error
  }
}
```

### Atomic Transaction Execution

Execute multiple operations atomically:

```graphql
mutation TransferFunds {
  executeTransaction(
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
      {
        opType: INSERT
        table: "transactions"
        data: {
          from_account: 1
          to_account: 2
          amount: 100
        }
      }
    ]
    isolationLevel: SERIALIZABLE
  ) {
    success
    transactionId
    executionTimeMs
    error
  }
}
```

**Isolation Levels**:
- `READ_UNCOMMITTED` - Lowest isolation
- `READ_COMMITTED` - Default
- `REPEATABLE_READ` - Consistent snapshot
- `SERIALIZABLE` - Highest isolation

---

## Subscription Operations

### WebSocket Protocol

Subscriptions use the `graphql-ws` protocol over WebSocket.

**Connection**: `ws://localhost:8080/graphql/ws`

### Table Change Subscription

```graphql
subscription WatchUserChanges {
  tableChanges(
    table: "users"
    operations: [INSERT, UPDATE, DELETE]
  ) {
    operation
    table
    row {
      id
      fields
    }
    timestamp
  }
}
```

### Row-Level Subscriptions

```graphql
subscription WatchNewUsers {
  rowInserted(table: "users") {
    id
    fields
    timestamp
  }
}

subscription WatchUserUpdates {
  rowUpdated(table: "users") {
    id
    oldValues
    newValues
    timestamp
  }
}

subscription WatchUserDeletions {
  rowDeleted(table: "users") {
    id
    fields
    timestamp
  }
}
```

### Aggregate Change Subscription

```graphql
subscription WatchUserCount {
  aggregateChanges(
    table: "users"
    aggregation: {
      function: COUNT
      field: "id"
    }
  ) {
    value
    timestamp
  }
}
```

### System Metrics Subscription

```graphql
subscription WatchMetrics {
  systemMetrics {
    cpu
    memory
    activeConnections
    queriesPerSecond
    timestamp
  }
}
```

### Heartbeat

```graphql
subscription {
  heartbeat {
    timestamp
    serverTime
  }
}
```

---

## Type System

### Filter Operators

```graphql
enum Operator {
  EQ          # Equal
  NE          # Not equal
  LT          # Less than
  LE          # Less than or equal
  GT          # Greater than
  GE          # Greater than or equal
  LIKE        # Pattern match
  NOT_LIKE    # Negative pattern
  IN          # In list
  NOT_IN      # Not in list
  IS_NULL     # Is null
  IS_NOT_NULL # Is not null
  BETWEEN     # Between range
  CONTAINS    # Contains substring
  STARTS_WITH # Starts with
  ENDS_WITH   # Ends with
}
```

### Aggregate Functions

```graphql
enum AggregateFunction {
  COUNT       # Count rows
  SUM         # Sum values
  AVG         # Average
  MIN         # Minimum
  MAX         # Maximum
  STD_DEV     # Standard deviation
  VARIANCE    # Variance
}
```

### Data Types

```graphql
enum DataType {
  NULL
  BOOLEAN
  INTEGER
  FLOAT
  STRING
  BYTES
  DATE
  TIMESTAMP
  JSON
  ARRAY
  DECIMAL
  UUID
}
```

### Join Types

```graphql
enum JoinType {
  INNER  # Inner join
  LEFT   # Left outer join
  RIGHT  # Right outer join
  FULL   # Full outer join
  CROSS  # Cross join
}
```

### Change Types

```graphql
enum ChangeType {
  INSERT
  UPDATE
  DELETE
  TRUNCATE
}
```

---

## Error Handling

### Union Types for Results

All operations return union types for proper error handling:

```graphql
union QueryResult = QuerySuccess | QueryError

type QuerySuccess {
  rows: [Row!]!
  totalCount: Int!
  executionTimeMs: Float
}

type QueryError {
  message: String!
  code: String!
  details: String
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `PERMISSION_DENIED` | Insufficient permissions |
| `NOT_FOUND` | Resource not found |
| `INVALID_ARGUMENT` | Invalid argument |
| `ALREADY_EXISTS` | Resource exists |
| `CONSTRAINT_VIOLATION` | Constraint violated |
| `TRANSACTION_ABORTED` | Transaction aborted |
| `INTERNAL_ERROR` | Internal error |

### Example Error Handling

```graphql
query {
  queryTable(table: "users") {
    __typename
    ... on QuerySuccess {
      rows { id fields }
    }
    ... on QueryError {
      message
      code
      details
    }
  }
}
```

**Error Response**:
```json
{
  "data": {
    "queryTable": {
      "__typename": "QueryError",
      "message": "Table 'users' not found",
      "code": "NOT_FOUND",
      "details": "Schema: public"
    }
  }
}
```

---

## Performance & Best Practices

### Query Optimization

**1. Use Field Selection**:
```graphql
# Bad: Select all fields
query {
  tables {
    name
    columns { name dataType nullable defaultValue }
  }
}

# Good: Select only needed fields
query {
  tables {
    name
  }
}
```

**2. Use Pagination**:
```graphql
query {
  queryTable(
    table: "users"
    limit: 50
    offset: 0
  ) {
    ... on QuerySuccess {
      rows { id fields }
      totalCount
    }
  }
}
```

**3. Use Indexes**:
```graphql
# Create indexes for frequently queried fields
mutation {
  createIndex(
    table: "users"
    name: "users_email_idx"
    columns: ["email"]
  ) {
    success
  }
}
```

**4. Batch Operations**:
```graphql
# Insert multiple rows in one operation
mutation {
  insertMany(
    table: "users"
    data: [
      { name: "User 1" }
      { name: "User 2" }
      { name: "User 3" }
    ]
  ) {
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

### Security Best Practices

**1. Use Parameterized Queries**:
Always use GraphQL variables instead of string interpolation.

**2. Limit Complexity**:
Server enforces complexity limits (max: 1000 points).

**3. Limit Depth**:
Server enforces depth limits (max: 10 levels).

**4. Rate Limiting**:
1000 complexity points per minute per user.

### Subscription Best Practices

**1. Unsubscribe Properly**:
```javascript
const subscription = client.subscribe({
  query: WATCH_USERS
});

// Later...
subscription.unsubscribe();
```

**2. Filter Server-Side**:
```graphql
subscription {
  tableChanges(
    table: "users"
    operations: [INSERT]  # Only watch inserts
  ) {
    operation
    row { id fields }
  }
}
```

**3. Limit Concurrent Subscriptions**:
Limit to 10 concurrent subscriptions per connection.

---

## Testing with cURL

### Run Test Suite

```bash
# Execute all GraphQL tests
./graphql_curl_commands.sh

# Run specific test categories
./graphql_curl_commands.sh queries
./graphql_curl_commands.sh mutations
./graphql_curl_commands.sh schema
```

### Individual Tests

**Query Test**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ tables { name rowCount } }"}'
```

**Mutation Test**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { insertOne(table: \"users\", data: { name: \"Test\" }) { __typename ... on MutationSuccess { affectedRows } } }"
  }'
```

---

## Additional Resources

- **GraphQL Specification**: https://spec.graphql.org/
- **GraphQL-WS Protocol**: https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md
- **GraphQL Playground**: `http://localhost:8080/graphql`
- **API Overview**: [API_OVERVIEW.md](./API_OVERVIEW.md)
- **WebSocket API**: [WEBSOCKET_API.md](./WEBSOCKET_API.md)

---

**Last Updated**: 2025-12-28
**GraphQL Version**: June 2018 Standard
**Product Version**: RustyDB v0.6.0 Enterprise Server
