# RustyDB GraphQL API Reference

**RustyDB v0.6.5 - Enterprise Server ($856M Release)**
**GraphQL Version**: June 2018
**Last Updated**: 2025-12-29
**Endpoint**: `http://localhost:8080/graphql`

> **Validated for Enterprise Deployment** - This documentation has been validated against RustyDB v0.6.5 production builds and is certified for enterprise use.

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Authentication](#authentication)
4. [Query Operations](#query-operations)
5. [Mutation Operations](#mutation-operations)
6. [Subscription Operations](#subscription-operations)
7. [Transaction Support](#transaction-support)
8. [Schema Introspection](#schema-introspection)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)

---

## Overview

RustyDB provides a full-featured GraphQL API for flexible, efficient database operations. The GraphQL interface enables:

- **Type-safe Queries**: Strong typing with schema validation
- **Flexible Data Fetching**: Request exactly the data you need
- **Real-time Subscriptions**: Live data updates via WebSocket
- **Batching & Caching**: Efficient multi-operation requests
- **Transaction Support**: ACID-compliant operations

### Key Features

- Complete CRUD operations
- Advanced filtering and sorting
- Cursor-based pagination
- Aggregate functions
- Full-text search
- Transaction management
- Real-time subscriptions
- Schema introspection

### Endpoint

**URL**: `POST http://localhost:8080/graphql`
**WebSocket**: `ws://localhost:8080/graphql/ws` (for subscriptions)
**GraphQL Playground**: `GET http://localhost:8080/graphql`

---

## Quick Start

### Test with curl

```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ schemas { name } }"}'
```

### GraphQL Playground

Visit `http://localhost:8080/graphql` in your browser to access the interactive GraphQL Playground with:

- Auto-completion
- Schema documentation
- Query history
- Variable editor
- Real-time error highlighting

---

## Authentication

GraphQL API authentication is **not required** in development mode. For production deployments, enable JWT authentication.

### Production Authentication

```graphql
# Add Authorization header
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Obtain Token

Use the REST API login endpoint:

```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'
```

---

## Query Operations

### List All Schemas

```graphql
{
  schemas {
    name
    tables {
      name
      rowCount
    }
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
        "tables": [
          {"name": "users", "rowCount": 10000},
          {"name": "orders", "rowCount": 50000}
        ]
      }
    ]
  }
}
```

### Get Table Details

```graphql
{
  table(name: "users", schema: "public") {
    name
    schema
    rowCount
    columns {
      name
      dataType
      nullable
      primaryKey
      defaultValue
    }
    indexes {
      name
      columns
      unique
      indexType
    }
  }
}
```

**Response**:
```json
{
  "data": {
    "table": {
      "name": "users",
      "schema": "public",
      "rowCount": 10000,
      "columns": [
        {
          "name": "id",
          "dataType": "INTEGER",
          "nullable": false,
          "primaryKey": true,
          "defaultValue": "nextval('users_id_seq')"
        },
        {
          "name": "name",
          "dataType": "VARCHAR(255)",
          "nullable": false,
          "primaryKey": false,
          "defaultValue": null
        }
      ],
      "indexes": [
        {
          "name": "users_pkey",
          "columns": ["id"],
          "unique": true,
          "indexType": "BTREE"
        }
      ]
    }
  }
}
```

### Query Table Data with Filters

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
    orderBy: [
      { field: "name", order: ASC }
    ]
    limit: 10
  ) {
    __typename
    ... on QuerySuccess {
      rows {
        id
        tableName
        fields
      }
      totalCount
      executionTimeMs
      hasMore
    }
    ... on QueryError {
      message
      code
      details
    }
  }
}
```

**Response**:
```json
{
  "data": {
    "queryTable": {
      "__typename": "QuerySuccess",
      "rows": [
        {
          "id": "1",
          "tableName": "users",
          "fields": {"id": 1, "name": "Alice", "age": 30}
        },
        {
          "id": "2",
          "tableName": "users",
          "fields": {"id": 2, "name": "Bob", "age": 28}
        }
      ],
      "totalCount": 2,
      "executionTimeMs": 12.5,
      "hasMore": false
    }
  }
}
```

### Complex Filters (AND/OR)

```graphql
{
  queryTable(
    table: "users"
    whereClause: {
      and: [
        { condition: { field: "age", operator: GE, value: "18" } }
        {
          or: [
            { condition: { field: "city", operator: EQ, value: "New York" } }
            { condition: { field: "city", operator: EQ, value: "San Francisco" } }
          ]
        }
      ]
    }
    limit: 20
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
    }
    ... on QueryError {
      message
      code
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
    whereClause: {
      condition: {
        field: "users.active"
        operator: EQ
        value: "true"
      }
    }
    limit: 50
  ) {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
      executionTimeMs
    }
    ... on QueryError {
      message
      code
    }
  }
}
```

### Cursor-Based Pagination

```graphql
{
  queryTableConnection(
    table: "users"
    first: 10
    after: "cursor-value"
  ) {
    __typename
    ... on QuerySuccess {
      rows {
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
    ... on QueryError {
      message
      code
    }
  }
}
```

### Aggregate Queries

```graphql
{
  aggregate(
    table: "orders"
    aggregates: [
      { func: COUNT, field: "id", alias: "total_orders" }
      { func: SUM, field: "amount", alias: "total_revenue" }
      { func: AVG, field: "amount", alias: "avg_order_value" }
      { func: MIN, field: "amount", alias: "min_order" }
      { func: MAX, field: "amount", alias: "max_order" }
    ]
    whereClause: {
      condition: {
        field: "status"
        operator: EQ
        value: "completed"
      }
    }
    groupBy: ["customer_id"]
  ) {
    results {
      groupValues
      aggregates
    }
    totalCount
    executionTimeMs
  }
}
```

**Response**:
```json
{
  "data": {
    "aggregate": {
      "results": [
        {
          "groupValues": ["customer_123"],
          "aggregates": {
            "total_orders": 15,
            "total_revenue": 1234.56,
            "avg_order_value": 82.30,
            "min_order": 12.50,
            "max_order": 234.99
          }
        }
      ],
      "totalCount": 1,
      "executionTimeMs": 23.4
    }
  }
}
```

### Count Query

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

**Response**:
```json
{
  "data": {
    "count": 8547
  }
}
```

### Full-Text Search

```graphql
{
  search(
    query: "john doe"
    tables: ["users", "customers"]
    fields: ["name", "email", "description"]
    limit: 20
  ) {
    results {
      table
      rowId
      score
      matchedFields
    }
    totalCount
    executionTimeMs
  }
}
```

### Explain Query Plan

```graphql
{
  explain(
    table: "users"
    whereClause: {
      condition: {
        field: "email"
        operator: LIKE
        value: "%@example.com"
      }
    }
    orderBy: [
      { field: "created_at", order: DESC }
    ]
  ) {
    planText
    estimatedCost
    estimatedRows
    operations {
      operationType
      tableName
      indexName
      estimatedCost
      estimatedRows
    }
  }
}
```

---

## Mutation Operations

### Insert Single Row

```graphql
mutation {
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
        createdAt
      }
      executionTimeMs
    }
    ... on MutationError {
      message
      code
      details
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
            "id": 123,
            "name": "John Doe",
            "email": "john@example.com",
            "age": 30
          },
          "createdAt": "2025-12-29T10:00:00Z"
        }
      ],
      "executionTimeMs": 5.2
    }
  }
}
```

### Insert Multiple Rows

```graphql
mutation {
  insertMany(
    table: "users"
    data: [
      { name: "Alice", email: "alice@example.com", age: 25 }
      { name: "Bob", email: "bob@example.com", age: 28 }
      { name: "Charlie", email: "charlie@example.com", age: 32 }
    ]
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning { id fields }
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### Update Single Row

```graphql
mutation {
  updateOne(
    table: "users"
    id: "123"
    data: {
      name: "John Updated"
      email: "john.updated@example.com"
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning {
        id
        fields
        updatedAt
        version
      }
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

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
      archived_at: "2025-12-29T00:00:00Z"
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### Delete Single Row

```graphql
mutation {
  deleteOne(
    table: "users"
    id: "123"
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### Delete Multiple Rows

```graphql
mutation {
  deleteMany(
    table: "users"
    whereClause: {
      condition: {
        field: "last_login"
        operator: LT
        value: "2024-01-01T00:00:00Z"
      }
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### Upsert (Insert or Update)

```graphql
mutation {
  upsert(
    table: "users"
    data: {
      id: 1
      name: "John Doe"
      email: "john@example.com"
    }
    conflictColumns: ["id"]
    updateColumns: ["name", "email"]
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning { id fields }
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### Bulk Insert

```graphql
mutation {
  bulkInsert(
    table: "logs"
    data: [
      { timestamp: "2025-12-29T10:00:00Z", level: "INFO", message: "Server started" }
      { timestamp: "2025-12-29T10:01:00Z", level: "DEBUG", message: "Processing request" }
      # ... thousands more
    ]
    batchSize: 1000
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### DDL Operations

#### Create Table

```graphql
mutation {
  createTable(
    name: "products"
    columns: [
      { name: "id", dataType: INTEGER, primaryKey: true, autoIncrement: true }
      { name: "name", dataType: VARCHAR, nullable: false }
      { name: "price", dataType: DECIMAL, nullable: false }
    ]
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
      executionTimeMs
    }
    ... on DdlError {
      message
      code
      details
    }
  }
}
```

#### Alter Table - Add Column

```graphql
mutation {
  alterTableAddColumn(
    table: "users"
    columnDefinition: {
      name: "phone"
      dataType: VARCHAR
      nullable: true
    }
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
      executionTimeMs
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

#### Create Index

```graphql
mutation {
  createIndex(
    table: "users"
    indexName: "idx_email"
    columns: ["email"]
    unique: true
    ifNotExists: true
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
      executionTimeMs
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

---

## Transaction Support

### Begin Transaction

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
      "timestamp": "2025-12-29T10:00:00.000Z",
      "isolationLevel": "SERIALIZABLE"
    }
  }
}
```

**Isolation Levels**:
- `READ_UNCOMMITTED`: Allows dirty reads
- `READ_COMMITTED`: Default, sees only committed data
- `REPEATABLE_READ`: Consistent snapshot per transaction
- `SERIALIZABLE`: Strictest isolation, full serializability

### Commit Transaction

```graphql
mutation {
  commitTransaction(transactionId: "88790068-3f05-42fb-a5f8-126ccedff088") {
    success
    transactionId
    error
  }
}
```

### Rollback Transaction

```graphql
mutation {
  rollbackTransaction(transactionId: "88790068-3f05-42fb-a5f8-126ccedff088") {
    success
    transactionId
    error
  }
}
```

### Execute Atomic Transaction

Execute multiple operations atomically in a single transaction.

```graphql
mutation {
  executeTransaction(
    operations: [
      {
        operationType: INSERT
        table: "accounts"
        data: { id: 1, balance: 1000 }
      }
      {
        operationType: UPDATE
        table: "accounts"
        where: { id: 2 }
        data: { balance: 500 }
      }
      {
        operationType: INSERT
        table: "transactions"
        data: { from_account: 1, to_account: 2, amount: 500 }
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

**Response**:
```json
{
  "data": {
    "executeTransaction": {
      "success": true,
      "transactionId": "txn_12345",
      "executionTimeMs": 0.002826,
      "error": null
    }
  }
}
```

---

## Subscription Operations

**Note**: Subscriptions require WebSocket connection (`ws://localhost:8080/graphql/ws`).

### Subscribe to Table Changes

```graphql
subscription {
  tableChanges(
    table: "users"
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "true"
      }
    }
  ) {
    changeType
    tableName
    rowId
    oldData
    newData
    timestamp
  }
}
```

**Event Stream**:
```json
{
  "data": {
    "tableChanges": {
      "changeType": "INSERT",
      "tableName": "users",
      "rowId": "124",
      "oldData": null,
      "newData": {"id": 124, "name": "New User", "active": true},
      "timestamp": "2025-12-29T10:05:00Z"
    }
  }
}
```

### Subscribe to Row Insertions

```graphql
subscription {
  rowInserted(table: "orders") {
    tableName
    row {
      id
      fields
      createdAt
    }
    timestamp
  }
}
```

### Subscribe to Row Updates

```graphql
subscription {
  rowUpdated(table: "users") {
    tableName
    rowId
    oldData
    newData
    changedFields
    timestamp
  }
}
```

### Subscribe to Row Deletions

```graphql
subscription {
  rowDeleted(table: "users") {
    tableName
    rowId
    deletedData
    timestamp
  }
}
```

### Subscribe to Specific Row Changes

```graphql
subscription {
  rowChanges(table: "users", id: "123") {
    changeType
    tableName
    rowId
    oldData
    newData
    timestamp
  }
}
```

### Subscribe to Aggregate Changes

```graphql
subscription {
  aggregateChanges(
    table: "orders"
    aggregates: [
      { func: COUNT, field: "id" }
      { func: SUM, field: "amount" }
    ]
    intervalSeconds: 5
  ) {
    aggregates
    previousAggregates
    timestamp
  }
}
```

### Subscribe to Query Result Changes

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
    orderBy: [{ field: "created_at", order: DESC }]
    limit: 10
    pollIntervalSeconds: 2
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

### Heartbeat Subscription

```graphql
subscription {
  heartbeat(intervalSeconds: 30) {
    timestamp
    serverTime
    connectionId
  }
}
```

---

## Schema Introspection

### Get All Available Queries

```graphql
{
  __type(name: "QueryRoot") {
    fields {
      name
      description
      args {
        name
        type { name kind }
      }
    }
  }
}
```

### Get All Available Mutations

```graphql
{
  __type(name: "MutationRoot") {
    fields {
      name
      description
      args {
        name
        type { name kind }
      }
    }
  }
}
```

### Get All Available Subscriptions

```graphql
{
  __type(name: "SubscriptionRoot") {
    fields {
      name
      description
      args {
        name
        type { name kind }
      }
    }
  }
}
```

### Full Schema Query

```graphql
{
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      name
      kind
      description
    }
  }
}
```

---

## Error Handling

All operations return union types for proper error handling:

```graphql
{
  __typename
  ... on Success {
    # Success fields
  }
  ... on Error {
    message
    code
    details
  }
}
```

### Common Error Codes

- **PERMISSION_DENIED**: Insufficient permissions
- **NOT_FOUND**: Resource not found
- **INVALID_ARGUMENT**: Invalid argument provided
- **ALREADY_EXISTS**: Resource already exists
- **CONSTRAINT_VIOLATION**: Database constraint violated
- **TRANSACTION_ABORTED**: Transaction was aborted
- **INTERNAL_ERROR**: Internal server error

### Error Response Example

```json
{
  "errors": [
    {
      "message": "Table 'users' not found",
      "locations": [{"line": 2, "column": 3}],
      "path": ["table"],
      "extensions": {
        "code": "NOT_FOUND",
        "details": {
          "table": "users",
          "schema": "public"
        }
      }
    }
  ]
}
```

---

## Best Practices

### 1. Use Fragments for Reusability

```graphql
fragment UserFields on Row {
  id
  fields
  createdAt
  updatedAt
}

query {
  queryTable(table: "users") {
    __typename
    ... on QuerySuccess {
      rows {
        ...UserFields
      }
    }
  }
}
```

### 2. Always Handle Errors

```graphql
{
  queryTable(table: "users") {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
    }
    ... on QueryError {
      message
      code
      details
    }
  }
}
```

### 3. Use Variables for Dynamic Queries

```graphql
query GetUsersByAge($minAge: Int!) {
  queryTable(
    table: "users"
    whereClause: {
      condition: {
        field: "age"
        operator: GT
        value: $minAge
      }
    }
  ) {
    ... on QuerySuccess {
      rows { id fields }
    }
  }
}
```

Variables:
```json
{
  "minAge": 18
}
```

### 4. Limit Result Sets

Always use `limit` for large tables:

```graphql
{
  queryTable(table: "users", limit: 100) {
    ... on QuerySuccess {
      rows { id fields }
      hasMore
    }
  }
}
```

### 5. Use Pagination for Large Datasets

```graphql
{
  queryTableConnection(
    table: "users"
    first: 50
    after: $cursor
  ) {
    ... on QuerySuccess {
      rows {
        edges {
          cursor
          node { id fields }
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
}
```

### 6. Batch Multiple Operations

```graphql
mutation BatchOperations {
  user1: insertOne(table: "users", data: {...}) { ... }
  user2: insertOne(table: "users", data: {...}) { ... }
  user3: insertOne(table: "users", data: {...}) { ... }
}
```

### 7. Use Transactions for Multi-Step Operations

```graphql
mutation {
  executeTransaction(
    operations: [
      { operationType: INSERT, table: "orders", data: {...} }
      { operationType: UPDATE, table: "inventory", data: {...} }
    ]
    isolationLevel: SERIALIZABLE
  ) {
    success
    error
  }
}
```

---

## Performance Tips

1. **Request Only Needed Fields**: Don't fetch unnecessary data
2. **Use Indexes**: Create indexes on frequently queried columns
3. **Use Aggregations**: Use aggregate functions instead of fetching all rows
4. **Limit Results**: Always use `limit` for large tables
5. **Use Explain**: Analyze query plans with `explain` query
6. **Batch Operations**: Combine multiple operations in single request
7. **Use Subscriptions Wisely**: Limit concurrent subscriptions
8. **Cache Results**: Implement client-side caching for static data

---

## Filter Operators Reference

| Operator | Description | Example |
|----------|-------------|---------|
| `EQ` | Equal | `age = 25` |
| `NE` | Not equal | `age != 25` |
| `LT` | Less than | `age < 25` |
| `LE` | Less than or equal | `age <= 25` |
| `GT` | Greater than | `age > 25` |
| `GE` | Greater than or equal | `age >= 25` |
| `LIKE` | Pattern match | `name LIKE '%John%'` |
| `NOT_LIKE` | Negative pattern | `name NOT LIKE '%test%'` |
| `IN` | In list | `status IN ['active', 'pending']` |
| `NOT_IN` | Not in list | `status NOT IN ['deleted']` |
| `IS_NULL` | Is null | `email IS NULL` |
| `IS_NOT_NULL` | Is not null | `email IS NOT NULL` |
| `BETWEEN` | Between range | `age BETWEEN 18 AND 65` |
| `CONTAINS` | Contains substring | `description CONTAINS 'test'` |
| `STARTS_WITH` | Starts with | `name STARTS_WITH 'John'` |
| `ENDS_WITH` | Ends with | `email ENDS_WITH '.com'` |

---

## Aggregate Functions Reference

| Function | Description | Example |
|----------|-------------|---------|
| `COUNT` | Count rows | `COUNT(id)` |
| `SUM` | Sum values | `SUM(amount)` |
| `AVG` | Average | `AVG(price)` |
| `MIN` | Minimum | `MIN(age)` |
| `MAX` | Maximum | `MAX(salary)` |
| `STD_DEV` | Standard deviation | `STD_DEV(score)` |
| `VARIANCE` | Variance | `VARIANCE(value)` |

---

## Additional Resources

- [GraphQL Specification](https://spec.graphql.org/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)
- [REST API Reference](./REST_API.md)
- [WebSocket API Reference](./WEBSOCKET_API.md)
- [GraphQL Examples](/home/user/rusty-db/docs/graphql_examples.md)
- [GraphQL Quick Reference](/home/user/rusty-db/docs/GRAPHQL_QUICK_REFERENCE.md)

---

**Validated for Enterprise Deployment** - RustyDB v0.6.5 ($856M Release)

*Last Updated: 2025-12-29*
*Documentation Version: 1.0.0*
