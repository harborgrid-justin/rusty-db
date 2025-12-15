# RustyDB GraphQL API - Quick Reference

**Endpoint**: `http://localhost:8080/graphql`
**Method**: POST
**Content-Type**: application/json

---

## Quick Start

### Test with curl
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ schemas { name } }"}'
```

### Run Test Suite
```bash
# Run all tests
./graphql_curl_commands.sh

# Run specific test suite
./graphql_curl_commands.sh queries
./graphql_curl_commands.sh mutations
./graphql_curl_commands.sh schema
```

---

## Core Operations at a Glance

### Queries (14 total)

| Operation | Purpose | Auth Required |
|-----------|---------|---------------|
| `schemas` | List all schemas | ❌ |
| `schema(name)` | Get schema details | ❌ |
| `tables` | List all tables | ❌ |
| `table(name)` | Get table details | ❌ |
| `queryTable` | Query table data | ❌ |
| `queryTables` | Query with joins | ❌ |
| `queryTableConnection` | Paginated query | ❌ |
| `row(table, id)` | Get single row | ❌ |
| `aggregate` | Aggregate queries | ❌ |
| `count` | Count rows | ❌ |
| `executeSql` | Run raw SQL | ✅ |
| `search` | Full-text search | ❌ |
| `explain` | Query plan | ❌ |
| `executeUnion` | Union queries | ❌ |

### Mutations (30 total)

| Operation | Purpose | Auth Required |
|-----------|---------|---------------|
| `insertOne` | Insert single row | ❌ |
| `insertMany` | Insert multiple rows | ❌ |
| `updateOne` | Update by ID | ❌ |
| `updateMany` | Bulk update | ❌ |
| `deleteOne` | Delete by ID | ❌ |
| `deleteMany` | Bulk delete | ❌ |
| `upsert` | Insert or update | ❌ |
| `bulkInsert` | High-volume insert | ❌ |
| `beginTransaction` | Start transaction | ❌ |
| `commitTransaction` | Commit transaction | ❌ |
| `rollbackTransaction` | Rollback transaction | ❌ |
| `executeTransaction` | Atomic multi-op | ❌ |
| `createDatabase` | Create database | ✅ |
| `dropDatabase` | Drop database | ✅ |
| `backupDatabase` | Backup database | ✅ |
| `alterTableAddColumn` | Add column | ❌ |
| `alterTableDropColumn` | Drop column | ❌ |
| `alterTableModifyColumn` | Modify column | ❌ |
| `alterTableAddConstraint` | Add constraint | ❌ |
| `alterTableDropConstraint` | Drop constraint | ❌ |
| `truncateTable` | Truncate table | ❌ |
| `createView` | Create view | ❌ |
| `dropView` | Drop view | ❌ |
| `createIndex` | Create index | ❌ |
| `dropIndex` | Drop index | ❌ |
| `createProcedure` | Create procedure | ❌ |
| `executeProcedure` | Execute procedure | ❌ |
| `insertIntoSelect` | INSERT INTO SELECT | ❌ |
| `selectInto` | SELECT INTO | ❌ |
| `executeStringFunction` | String operation | ❌ |
| `batchStringFunctions` | Batch string ops | ❌ |

### Subscriptions (8 total)

| Operation | Purpose | Protocol |
|-----------|---------|----------|
| `tableChanges` | Table change events | WebSocket |
| `rowInserted` | Row insert events | WebSocket |
| `rowUpdated` | Row update events | WebSocket |
| `rowDeleted` | Row delete events | WebSocket |
| `rowChanges` | Specific row changes | WebSocket |
| `aggregateChanges` | Aggregate changes | WebSocket |
| `queryChanges` | Query result changes | WebSocket |
| `heartbeat` | Connection heartbeat | WebSocket |

---

## Common Patterns

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
    ... on QueryError {
      message
      code
    }
  }
}
```

### Insert Row
```graphql
mutation {
  insertOne(
    table: "users"
    data: {
      name: "John"
      email: "john@example.com"
      age: 30
    }
  ) {
    __typename
    ... on MutationSuccess {
      affectedRows
      returning { id fields }
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

### Update with Filter
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
    ... on MutationError {
      message
      code
    }
  }
}
```

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

## Aggregate Functions

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

## Data Types

| Type | Description | Example |
|------|-------------|---------|
| `NULL` | Null value | `null` |
| `BOOLEAN` | True/False | `true`, `false` |
| `INTEGER` | Integer number | `42` |
| `FLOAT` | Floating point | `3.14` |
| `STRING` | Text string | `"hello"` |
| `BYTES` | Binary data | `0x1234` |
| `DATE` | Date only | `"2025-12-11"` |
| `TIMESTAMP` | Date and time | `"2025-12-11T10:30:00Z"` |
| `JSON` | JSON object | `{"key": "value"}` |
| `ARRAY` | Array | `[1, 2, 3]` |
| `DECIMAL` | Decimal number | `123.45` |
| `UUID` | UUID | `"550e8400-e29b-41d4-a716-446655440000"` |

---

## Isolation Levels

| Level | Description |
|-------|-------------|
| `READ_UNCOMMITTED` | Lowest isolation, allows dirty reads |
| `READ_COMMITTED` | Prevents dirty reads |
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

- `PERMISSION_DENIED` - Insufficient permissions
- `NOT_FOUND` - Resource not found
- `INVALID_ARGUMENT` - Invalid argument
- `ALREADY_EXISTS` - Resource exists
- `CONSTRAINT_VIOLATION` - Constraint violated
- `TRANSACTION_ABORTED` - Transaction aborted
- `INTERNAL_ERROR` - Internal error

---

## Performance Tips

1. **Use Indexes**: Create indexes on frequently queried columns
2. **Limit Results**: Always use `limit` for large tables
3. **Use Aggregations**: Use `aggregate` instead of fetching all rows
4. **Batch Operations**: Use `insertMany`, `updateMany`, `deleteMany` for bulk ops
5. **Use Explain**: Use `explain` to analyze query performance
6. **Use Pagination**: Use `queryTableConnection` for cursor-based pagination
7. **Use Transactions**: Group related operations in transactions

---

## Examples

### Simple Query
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ tables { name rowCount } }"}'
```

### Filtered Query
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ queryTable(table: \"users\", whereClause: { condition: { field: \"age\", operator: GT, value: \"25\" } }, limit: 10) { __typename ... on QuerySuccess { rows { id fields } totalCount } ... on QueryError { message code } } }"}'
```

### Insert Mutation
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { insertOne(table: \"users\", data: { name: \"John\", email: \"john@example.com\" }) { __typename ... on MutationSuccess { affectedRows } ... on MutationError { message code } } }"}'
```

### Count
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ count(table: \"users\") }"}'
```

---

## Documentation Files

1. **graphql_test_results.md** - Detailed test results (90 tests)
2. **graphql_examples.md** - Complete query examples (56 examples)
3. **graphql_test_summary.md** - Executive summary
4. **graphql_curl_commands.sh** - Executable test script
5. **GRAPHQL_QUICK_REFERENCE.md** - This quick reference

---

## Additional Resources

- GraphQL Spec: https://spec.graphql.org/
- GraphQL Best Practices: https://graphql.org/learn/best-practices/
- RustyDB Documentation: (link TBD)

---

**Last Updated**: 2025-12-11
**API Version**: 1.0
**GraphQL Version**: June 2018
