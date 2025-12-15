# GraphQL DdlResult Type - Usage Guide

## Overview

`DdlResult` is a **GraphQL Union type** used for DDL (Data Definition Language) operations like creating databases, altering tables, managing indexes, etc.

## Type Definition

```graphql
union DdlResult = DdlSuccess | DdlError

type DdlSuccess {
  success: Boolean!          # Always true for success variant
  message: String!           # Human-readable success message
  affected_rows: Int!        # Number of objects affected (renamed from affected_objects)
  execution_time_ms: Float!  # Execution time in milliseconds
}

type DdlError {
  success: Boolean!       # Always false for error variant
  message: String!        # Human-readable error message
  code: String!           # Machine-readable error code
  details: String         # Optional detailed error information
}
```

## Field Changes

### Fixed Field Names:
- ✅ **RENAMED**: `affected_objects` → `affected_rows` (for consistency with MutationSuccess)
- ✅ **ADDED**: `success: Boolean!` field to both DdlSuccess and DdlError
- ✅ All fields now properly documented

## Correct Usage

### ✅ Correct Query Pattern (Using Fragments)

```graphql
mutation {
  createDatabase(name: "test_db", ifNotExists: true) {
    ... on DdlSuccess {
      success
      message
      affected_rows
      execution_time_ms
    }
    ... on DdlError {
      success
      message
      code
      details
    }
  }
}
```

### ✅ Minimal Fragment Query

```graphql
mutation {
  createDatabase(name: "test_db") {
    ... on DdlSuccess {
      message
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

### ✅ Using __typename

```graphql
mutation {
  createDatabase(name: "test_db") {
    __typename
    ... on DdlSuccess {
      message
      affected_rows
    }
    ... on DdlError {
      message
      code
    }
  }
}
```

## Common Mistakes

### ❌ WRONG: Querying Union Directly

```graphql
mutation {
  createDatabase(name: "test_db") {
    success      # ERROR: Union has no direct fields!
    message      # ERROR: Must use fragments!
  }
}
```

**Error**: `Unknown field "success" on type "DdlResult"`

### ❌ WRONG: Using Old Field Names

```graphql
mutation {
  createDatabase(name: "test_db") {
    ... on DdlSuccess {
      affected_objects  # ERROR: Field renamed to affected_rows
    }
  }
}
```

**Error**: `Unknown field "affected_objects" on type "DdlSuccess"`

## DDL Mutations That Return DdlResult

### Database Management
- `createDatabase(name: String!, ifNotExists: Boolean): DdlResult!`
- `dropDatabase(name: String!, ifExists: Boolean): DdlResult!`
- `backupDatabase(name: String!, location: String!, fullBackup: Boolean): DdlResult!`

### Table Management
- `alterTableAddColumn(table: String!, column: ColumnDefinitionInput!): DdlResult!`
- `alterTableDropColumn(table: String!, columnName: String!, ifExists: Boolean): DdlResult!`
- `alterTableModifyColumn(table: String!, column: ColumnDefinitionInput!): DdlResult!`
- `alterTableAddConstraint(table: String!, constraint: ConstraintInput!): DdlResult!`
- `alterTableDropConstraint(table: String!, constraintName: String!, ifExists: Boolean): DdlResult!`
- `truncateTable(table: String!): DdlResult!`

### View Management
- `createView(name: String!, query: String!, orReplace: Boolean): DdlResult!`
- `dropView(name: String!, ifExists: Boolean): DdlResult!`

### Index Management
- `createIndex(table: String!, indexName: String!, columns: [String!]!, unique: Boolean, ifNotExists: Boolean): DdlResult!`
- `dropIndex(indexName: String!, table: String, ifExists: Boolean): DdlResult!`

### Stored Procedure Management
- `createProcedure(name: String!, parameters: [ProcedureParameter!]!, body: String!, orReplace: Boolean): DdlResult!`

### Advanced Operations
- `selectInto(newTable: String!, sourceQuery: String!): DdlResult!`

## Examples

### Example 1: Create Database with Full Error Handling

```graphql
mutation CreateDB {
  createDatabase(name: "analytics_db", ifNotExists: true) {
    __typename
    ... on DdlSuccess {
      success
      message
      affected_rows
      execution_time_ms
    }
    ... on DdlError {
      success
      message
      code
      details
    }
  }
}
```

**Success Response:**
```json
{
  "data": {
    "createDatabase": {
      "__typename": "DdlSuccess",
      "success": true,
      "message": "Database 'analytics_db' created successfully",
      "affected_rows": 1,
      "execution_time_ms": 45.23
    }
  }
}
```

**Error Response:**
```json
{
  "data": {
    "createDatabase": {
      "__typename": "DdlError",
      "success": false,
      "message": "Permission denied",
      "code": "PERMISSION_DENIED",
      "details": "Requires admin.create_database permission"
    }
  }
}
```

### Example 2: Create Index

```graphql
mutation CreateIdx {
  createIndex(
    table: "users",
    indexName: "idx_email",
    columns: ["email"],
    unique: true,
    ifNotExists: true
  ) {
    ... on DdlSuccess {
      success
      message
      affected_rows
    }
    ... on DdlError {
      success
      message
      code
    }
  }
}
```

### Example 3: Alter Table

```graphql
mutation AddColumn {
  alterTableAddColumn(
    table: "users",
    column: {
      name: "middle_name",
      dataType: "VARCHAR(100)",
      nullable: true
    }
  ) {
    ... on DdlSuccess {
      message
      execution_time_ms
    }
    ... on DdlError {
      message
      code
      details
    }
  }
}
```

## Client-Side Handling (JavaScript/TypeScript)

```typescript
const mutation = gql`
  mutation CreateDatabase($name: String!) {
    createDatabase(name: $name) {
      __typename
      ... on DdlSuccess {
        success
        message
        affected_rows
      }
      ... on DdlError {
        success
        message
        code
      }
    }
  }
`;

// Handle response
const result = await client.mutate({ mutation, variables: { name: "test_db" } });
const data = result.data.createDatabase;

if (data.__typename === "DdlSuccess") {
  console.log(`✅ ${data.message}`);
  console.log(`Affected: ${data.affected_rows} objects`);
} else if (data.__typename === "DdlError") {
  console.error(`❌ ${data.message} (${data.code})`);
}

// Or using the success field
if (data.success) {
  console.log(`✅ Success: ${data.message}`);
} else {
  console.error(`❌ Error: ${data.message}`);
}
```

## Testing

### Curl Example

```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createDatabase(name: \"test_db\") { __typename ... on DdlSuccess { success message affected_rows } ... on DdlError { success message code } } }"
  }'
```

## Common Error Codes

| Code | Description |
|------|-------------|
| `PERMISSION_DENIED` | User lacks required permissions |
| `CREATE_DATABASE_ERROR` | Failed to create database |
| `DROP_DATABASE_ERROR` | Failed to drop database |
| `ALTER_TABLE_ERROR` | Failed to alter table |
| `CREATE_INDEX_ERROR` | Failed to create index |
| `DROP_INDEX_ERROR` | Failed to drop index |
| `CREATE_VIEW_ERROR` | Failed to create view |
| `DROP_VIEW_ERROR` | Failed to drop view |
| `TRUNCATE_ERROR` | Failed to truncate table |
| `BACKUP_ERROR` | Failed to backup database |
| `CREATE_PROCEDURE_ERROR` | Failed to create procedure |
| `SELECT_INTO_ERROR` | Failed to execute SELECT INTO |

## Migration Guide

If you have existing queries using the old field names, update them as follows:

### Before (Old - ❌ Broken)
```graphql
mutation {
  createDatabase(name: "test") {
    success message  # Won't work - no direct union fields
  }
}
```

### After (New - ✅ Works)
```graphql
mutation {
  createDatabase(name: "test") {
    ... on DdlSuccess { success message affected_rows }
    ... on DdlError { success message code }
  }
}
```

### Field Renaming
- `affected_objects` → `affected_rows`

## Summary

1. **DdlResult is a Union** - Must use inline fragments (`... on DdlSuccess`, `... on DdlError`)
2. **Use `success` field** - Both variants now have a `success: Boolean!` field
3. **Field renamed** - `affected_objects` is now `affected_rows`
4. **Check `__typename`** - Use this to determine which variant was returned
5. **Handle both cases** - Always provide fragments for both Success and Error variants

## Related Types

- **MutationResult**: Similar union for DML operations (INSERT, UPDATE, DELETE)
- **ProcedureResult**: Similar union for stored procedure execution
- **QueryResult**: Similar union for query operations

---

**Last Updated**: 2025-12-11
**Agent**: Agent 9 - PhD CS Engineer (GraphQL Schema Design)
