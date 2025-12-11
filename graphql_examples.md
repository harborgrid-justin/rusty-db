# RustyDB GraphQL API - Example Queries

Complete reference for all GraphQL operations with examples.

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

---

## Query Operations

### 1. List All Schemas
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

### 2. Get Specific Schema
```graphql
{
  schema(name: "public") {
    name
    tables {
      name
      rowCount
      columns {
        name
        dataType
        nullable
      }
    }
  }
}
```

### 3. List All Tables
```graphql
{
  tables(schema: "public", limit: 10, offset: 0) {
    name
    schema
    rowCount
    columns {
      name
      dataType
      nullable
      defaultValue
      primaryKey
    }
    indexes {
      name
      columns
      unique
    }
  }
}
```

### 4. Get Specific Table Details
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
      defaultValue
      primaryKey
      autoIncrement
    }
    indexes {
      name
      columns
      unique
      indexType
    }
    constraints {
      name
      constraintType
      columns
    }
    statistics {
      rowCount
      sizeBytes
      lastAnalyzed
    }
  }
}
```

### 5. Query Table Data (Basic)
```graphql
{
  queryTable(table: "users") {
    __typename
    ... on QuerySuccess {
      rows {
        id
        tableName
        fields
        createdAt
        updatedAt
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

### 6. Query Table with Filters
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
    offset: 0
  ) {
    __typename
    ... on QuerySuccess {
      rows {
        id
        fields
      }
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

### 7. Query Table with Complex Filters (AND/OR)
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
    orderBy: [
      { field: "age", order: DESC }
      { field: "name", order: ASC }
    ]
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

### 8. Query Tables with Joins
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

### 9. Query with Pagination (Connection)
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

### 10. Get Single Row by ID
```graphql
{
  row(table: "users", id: "123") {
    id
    tableName
    fields
    createdAt
    updatedAt
    createdBy
    updatedBy
    version
  }
}
```

### 11. Get Field Value from Row
```graphql
{
  row(table: "users", id: "123") {
    id
    getField(name: "email") {
      stringValue
      intValue
      floatValue
      boolValue
      dateValue
      jsonValue
      isNull
    }
  }
}
```

### 12. Aggregate Queries
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

### 13. Count Query
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

### 14. Execute Raw SQL (Requires Auth)
```graphql
{
  executeSql(sql: "SELECT * FROM users WHERE age > 25 LIMIT 10") {
    __typename
    ... on QuerySuccess {
      rows { id fields }
      totalCount
      executionTimeMs
    }
    ... on QueryError {
      message
      code
      details
    }
  }
}
```

### 15. Full-Text Search
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

### 16. Explain Query Plan
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

### 17. Execute Union Query
```graphql
{
  executeUnion(
    queries: [
      { table: "active_users", whereClause: { ... } }
      { table: "inactive_users", whereClause: { ... } }
    ]
    distinct: true
    orderBy: [{ field: "name", order: ASC }]
    limit: 100
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

---

## Mutation Operations

### 18. Insert One Row
```graphql
mutation {
  insertOne(
    table: "users"
    data: {
      id: 1
      name: "John Doe"
      email: "john@example.com"
      age: 30
      active: true
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

### 19. Insert Many Rows
```graphql
mutation {
  insertMany(
    table: "users"
    data: [
      { id: 1, name: "Alice", email: "alice@example.com" }
      { id: 2, name: "Bob", email: "bob@example.com" }
      { id: 3, name: "Charlie", email: "charlie@example.com" }
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

### 20. Update One Row
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
      returning { id fields updatedAt version }
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### 21. Update Many Rows
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
      archived_at: "2025-12-11T00:00:00Z"
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

### 22. Delete One Row
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

### 23. Delete Many Rows
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

### 24. Upsert (Insert or Update)
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

### 25. Bulk Insert
```graphql
mutation {
  bulkInsert(
    table: "logs"
    data: [
      { timestamp: "2025-12-11T10:00:00Z", level: "INFO", message: "Server started" }
      { timestamp: "2025-12-11T10:01:00Z", level: "DEBUG", message: "Processing request" }
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

### 26. Begin Transaction
```graphql
mutation {
  beginTransaction(isolationLevel: READ_COMMITTED) {
    transactionId
    status
    timestamp
  }
}
```

### 27. Commit Transaction
```graphql
mutation {
  commitTransaction(transactionId: "txn_123") {
    transactionId
    status
    timestamp
  }
}
```

### 28. Rollback Transaction
```graphql
mutation {
  rollbackTransaction(transactionId: "txn_123") {
    transactionId
    status
    timestamp
  }
}
```

### 29. Execute Transaction (All-in-One)
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
        whereClause: { condition: { field: "id", operator: EQ, value: "2" } }
        data: { balance: 500 }
      }
      {
        opType: INSERT
        table: "transactions"
        data: { from_account: 1, to_account: 2, amount: 500 }
      }
    ]
  ) {
    transactionId
    status
    results {
      success
      affectedRows
      error
    }
    timestamp
  }
}
```

### 30. Create Database (Requires Auth)
```graphql
mutation {
  createDatabase(name: "analytics_db") {
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

### 31. Drop Database (Requires Auth)
```graphql
mutation {
  dropDatabase(name: "old_db", ifExists: true) {
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

### 32. Backup Database (Requires Auth)
```graphql
mutation {
  backupDatabase(
    name: "production"
    backupPath: "/backups/prod_2025_12_11.bak"
    compressed: true
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

### 33. Alter Table - Add Column
```graphql
mutation {
  alterTableAddColumn(
    table: "users"
    columnDefinition: {
      name: "phone"
      dataType: VARCHAR
      nullable: true
      defaultValue: null
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

### 34. Alter Table - Drop Column
```graphql
mutation {
  alterTableDropColumn(
    table: "users"
    column: "legacy_field"
    ifExists: true
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 35. Alter Table - Modify Column
```graphql
mutation {
  alterTableModifyColumn(
    table: "users"
    columnDefinition: {
      name: "email"
      dataType: VARCHAR
      nullable: false
    }
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 36. Alter Table - Add Constraint
```graphql
mutation {
  alterTableAddConstraint(
    table: "users"
    constraint: {
      name: "uk_email"
      constraintType: UNIQUE
      columns: ["email"]
    }
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 37. Alter Table - Drop Constraint
```graphql
mutation {
  alterTableDropConstraint(
    table: "users"
    constraintName: "uk_email"
    ifExists: true
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 38. Truncate Table
```graphql
mutation {
  truncateTable(table: "logs") {
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

### 39. Create View
```graphql
mutation {
  createView(
    name: "active_users_view"
    query: "SELECT id, name, email FROM users WHERE active = true"
    orReplace: true
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 40. Drop View
```graphql
mutation {
  dropView(name: "old_view", ifExists: true) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 41. Create Index
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

### 42. Drop Index
```graphql
mutation {
  dropIndex(
    table: "users"
    indexName: "idx_old"
    ifExists: true
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 43. Create Procedure
```graphql
mutation {
  createProcedure(
    name: "calculate_total"
    parameters: [
      { name: "user_id", dataType: INTEGER, mode: IN }
      { name: "total", dataType: DECIMAL, mode: OUT }
    ]
    body: "BEGIN SELECT SUM(amount) INTO total FROM orders WHERE user_id = user_id; END;"
    orReplace: true
  ) {
    __typename
    ... on DdlSuccess {
      message
      affectedObjects
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### 44. Execute Procedure
```graphql
mutation {
  executeProcedure(
    name: "calculate_total"
    parameters: [
      { name: "user_id", value: "123" }
    ]
  ) {
    __typename
    ... on ProcedureSuccess {
      outputParameters
      resultSets
      executionTimeMs
    }
    ... on ProcedureError {
      message
      code
    }
  }
}
```

### 45. Insert Into Select
```graphql
mutation {
  insertIntoSelect(
    targetTable: "archive_users"
    sourceTable: "users"
    columns: ["id", "name", "email"]
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "false"
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

### 46. Select Into (Create Table from Query)
```graphql
mutation {
  selectInto(
    newTable: "active_users_snapshot"
    sourceTable: "users"
    columns: ["id", "name", "email", "created_at"]
    whereClause: {
      condition: {
        field: "active"
        operator: EQ
        value: "true"
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

### 47. Execute String Function
```graphql
mutation {
  executeStringFunction(
    func: {
      functionType: UPPER
      input: "hello world"
    }
  ) {
    result
    functionType
  }
}
```

### 48. Batch String Functions
```graphql
mutation {
  batchStringFunctions(
    functions: [
      { functionType: UPPER, input: "hello" }
      { functionType: LOWER, input: "WORLD" }
      { functionType: TRIM, input: "  spaces  " }
    ]
  ) {
    results {
      result
      functionType
    }
    executionTimeMs
  }
}
```

---

## Subscription Operations

**Note**: Subscriptions require WebSocket connection. Use a GraphQL client that supports subscriptions (e.g., Apollo Client, graphql-ws).

### 49. Subscribe to Table Changes
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

### 50. Subscribe to Row Insertions
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

### 51. Subscribe to Row Updates
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

### 52. Subscribe to Row Deletions
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

### 53. Subscribe to Specific Row Changes
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

### 54. Subscribe to Aggregate Changes
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

### 55. Subscribe to Query Result Changes
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

### 56. Heartbeat Subscription
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

## Enum Values Reference

### AggregateFunc
- COUNT
- SUM
- AVG
- MIN
- MAX
- STD_DEV
- VARIANCE

### DataType
- NULL
- BOOLEAN
- INTEGER
- FLOAT
- STRING
- BYTES
- DATE
- TIMESTAMP
- JSON
- ARRAY
- DECIMAL
- UUID

### FilterOp
- EQ (equals)
- NE (not equals)
- LT (less than)
- LE (less than or equal)
- GT (greater than)
- GE (greater than or equal)
- LIKE (pattern match)
- NOT_LIKE (negative pattern match)
- IN (in list)
- NOT_IN (not in list)
- IS_NULL (is null)
- IS_NOT_NULL (is not null)
- BETWEEN (between range)
- CONTAINS (contains substring)
- STARTS_WITH (starts with prefix)
- ENDS_WITH (ends with suffix)

### IsolationLevel
- READ_UNCOMMITTED
- READ_COMMITTED
- REPEATABLE_READ
- SERIALIZABLE

### JoinType
- INNER
- LEFT
- RIGHT
- FULL
- CROSS

### SortOrder
- ASC (ascending)
- DESC (descending)

### ChangeType
- INSERT
- UPDATE
- DELETE

### ConstraintTypeEnum
- PRIMARY_KEY
- FOREIGN_KEY
- UNIQUE
- CHECK
- NOT_NULL

---

## Error Codes

Common error codes returned by the API:

- **PERMISSION_DENIED**: Insufficient permissions for the operation
- **NOT_FOUND**: Requested resource not found
- **INVALID_ARGUMENT**: Invalid argument provided
- **ALREADY_EXISTS**: Resource already exists
- **CONSTRAINT_VIOLATION**: Database constraint violated
- **TRANSACTION_ABORTED**: Transaction was aborted
- **INTERNAL_ERROR**: Internal server error

---

## Best Practices

1. **Use Fragments**: Define reusable fragments for complex queries
2. **Error Handling**: Always use union types with `__typename` for proper error handling
3. **Pagination**: Use `queryTableConnection` for large result sets
4. **Transactions**: Use transactions for multi-step operations that need atomicity
5. **Filtering**: Use `whereClause` with proper operators for efficient filtering
6. **Indexing**: Create indexes on frequently queried columns
7. **Monitoring**: Subscribe to relevant events for real-time monitoring

---

**Generated**: 2025-12-11
**Version**: 1.0
