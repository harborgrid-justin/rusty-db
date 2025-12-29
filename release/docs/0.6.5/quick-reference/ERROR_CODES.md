# RustyDB Error Codes Reference v0.6.5

**Document Version**: 1.0
**Product Version**: RustyDB 0.6.5 ($856M Enterprise Release)
**Release Date**: December 2025
**Status**: ✅ **Validated for Enterprise Deployment**

---

## Table of Contents

1. [Error Code Format](#error-code-format)
2. [HTTP Status Codes](#http-status-codes)
3. [Database Error Codes](#database-error-codes)
4. [SQL Error Codes](#sql-error-codes)
5. [Transaction Error Codes](#transaction-error-codes)
6. [Security Error Codes](#security-error-codes)
7. [Network Error Codes](#network-error-codes)
8. [Cluster Error Codes](#cluster-error-codes)
9. [GraphQL Error Codes](#graphql-error-codes)
10. [PL/SQL Error Codes](#plsql-error-codes)

---

## Error Code Format

### Standard Error Response

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "field": "additional_context",
      "value": "error_value"
    },
    "timestamp": "2025-12-29T10:00:00Z",
    "request_id": "req_12345",
    "stack_trace": "..."
  }
}
```

### Error Code Structure

```
CATEGORY_SPECIFIC_ERROR
Examples:
- DB_TABLE_NOT_FOUND
- TXN_DEADLOCK_DETECTED
- SEC_PERMISSION_DENIED
```

---

## HTTP Status Codes

### Success Codes (2xx)

| Code | Name | Description |
|------|------|-------------|
| `200` | OK | Request succeeded |
| `201` | Created | Resource created successfully |
| `202` | Accepted | Request accepted for processing |
| `204` | No Content | Request succeeded, no content returned |

### Client Error Codes (4xx)

| Code | Name | Description | Action |
|------|------|-------------|--------|
| `400` | Bad Request | Malformed request | Fix request syntax |
| `401` | Unauthorized | Authentication required | Provide valid credentials |
| `403` | Forbidden | Insufficient permissions | Request access or use different account |
| `404` | Not Found | Resource not found | Verify resource exists |
| `405` | Method Not Allowed | HTTP method not supported | Use correct HTTP method |
| `408` | Request Timeout | Request took too long | Retry request |
| `409` | Conflict | Resource conflict | Resolve conflict and retry |
| `410` | Gone | Resource permanently deleted | Resource no longer available |
| `413` | Payload Too Large | Request body too large | Reduce request size |
| `422` | Unprocessable Entity | Validation error | Fix validation errors |
| `429` | Too Many Requests | Rate limit exceeded | Wait and retry |

### Server Error Codes (5xx)

| Code | Name | Description | Action |
|------|------|-------------|--------|
| `500` | Internal Server Error | Server error | Contact support |
| `501` | Not Implemented | Feature not implemented | Use alternative method |
| `502` | Bad Gateway | Invalid response from upstream | Retry or contact support |
| `503` | Service Unavailable | Server overloaded or maintenance | Retry later |
| `504` | Gateway Timeout | Upstream timeout | Retry request |

---

## Database Error Codes

### General Database Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `DB_INTERNAL_ERROR` | 500 | Internal database error | Check logs, contact support |
| `DB_NOT_INITIALIZED` | 500 | Database not initialized | Initialize database |
| `DB_CORRUPTED` | 500 | Database corruption detected | Restore from backup |
| `DB_SHUTDOWN` | 503 | Database shutting down | Wait for shutdown to complete |
| `DB_READ_ONLY` | 403 | Database in read-only mode | Check configuration |
| `DB_FULL` | 507 | Database storage full | Free up disk space |

### Table Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `DB_TABLE_NOT_FOUND` | 404 | Table does not exist | Verify table name |
| `DB_TABLE_ALREADY_EXISTS` | 409 | Table already exists | Use different name or DROP existing table |
| `DB_TABLE_LOCKED` | 423 | Table is locked | Wait for lock release |
| `DB_TABLE_IN_USE` | 409 | Table in use by other sessions | Close other sessions or wait |

### Column Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `DB_COLUMN_NOT_FOUND` | 404 | Column does not exist | Verify column name |
| `DB_COLUMN_ALREADY_EXISTS` | 409 | Column already exists | Use different name |
| `DB_INVALID_COLUMN_TYPE` | 400 | Invalid column data type | Use valid data type |
| `DB_TYPE_MISMATCH` | 400 | Data type mismatch | Convert to correct type |

### Constraint Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `DB_CONSTRAINT_VIOLATION` | 409 | Constraint violated | Fix data to satisfy constraint |
| `DB_PRIMARY_KEY_VIOLATION` | 409 | Primary key constraint violated | Use unique primary key value |
| `DB_FOREIGN_KEY_VIOLATION` | 409 | Foreign key constraint violated | Ensure referenced row exists |
| `DB_UNIQUE_VIOLATION` | 409 | Unique constraint violated | Use unique value |
| `DB_CHECK_VIOLATION` | 409 | Check constraint violated | Provide valid value |
| `DB_NOT_NULL_VIOLATION` | 422 | NOT NULL constraint violated | Provide non-null value |

### Index Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `DB_INDEX_NOT_FOUND` | 404 | Index does not exist | Verify index name |
| `DB_INDEX_ALREADY_EXISTS` | 409 | Index already exists | Use different name or DROP existing |
| `DB_INDEX_CORRUPTED` | 500 | Index corruption detected | REINDEX table |

---

## SQL Error Codes

### Parse Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SQL_PARSE_ERROR` | 400 | SQL syntax error | Fix SQL syntax |
| `SQL_INVALID_STATEMENT` | 400 | Invalid SQL statement | Use valid SQL |
| `SQL_MISSING_FROM` | 400 | Missing FROM clause | Add FROM clause |
| `SQL_UNMATCHED_PARENTHESES` | 400 | Unmatched parentheses | Balance parentheses |
| `SQL_INVALID_KEYWORD` | 400 | Invalid or misspelled keyword | Use correct keyword |

### Execution Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SQL_EXECUTION_ERROR` | 500 | Query execution failed | Check query and data |
| `SQL_DIVISION_BY_ZERO` | 400 | Division by zero | Add zero check |
| `SQL_NUMERIC_OVERFLOW` | 400 | Numeric value overflow | Use smaller value or different type |
| `SQL_INVALID_CONVERSION` | 400 | Type conversion failed | Use valid conversion |
| `SQL_AMBIGUOUS_COLUMN` | 400 | Ambiguous column reference | Qualify with table name |

### Injection Prevention

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SQL_INJECTION_DETECTED` | 403 | SQL injection attempt detected | Use parameterized queries |
| `SQL_DANGEROUS_PATTERN` | 403 | Dangerous SQL pattern detected | Revise query |
| `SQL_UNAUTHORIZED_OPERATION` | 403 | Unauthorized operation | Check permissions |

---

## Transaction Error Codes

### Transaction Lifecycle

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `TXN_NOT_FOUND` | 404 | Transaction not found | Verify transaction ID |
| `TXN_ALREADY_COMMITTED` | 409 | Transaction already committed | Cannot modify committed transaction |
| `TXN_ALREADY_ABORTED` | 409 | Transaction already aborted | Start new transaction |
| `TXN_TIMEOUT` | 408 | Transaction timeout | Reduce transaction duration |
| `TXN_TOO_LONG` | 409 | Transaction exceeded max duration | Break into smaller transactions |

### Concurrency Control

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `TXN_DEADLOCK_DETECTED` | 409 | Deadlock detected | Retry transaction |
| `TXN_SERIALIZATION_FAILURE` | 409 | Serialization failure | Retry with lower isolation level |
| `TXN_LOCK_TIMEOUT` | 408 | Lock acquisition timeout | Retry or increase timeout |
| `TXN_LOCK_NOT_AVAILABLE` | 423 | Lock not available | Wait or retry |
| `TXN_SNAPSHOT_TOO_OLD` | 409 | Snapshot too old | Retry transaction |

### MVCC Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `TXN_VERSION_CONFLICT` | 409 | Version conflict | Retry transaction |
| `TXN_WRITE_CONFLICT` | 409 | Write conflict detected | Retry transaction |
| `TXN_READ_CONFLICT` | 409 | Read conflict detected | Retry transaction |

---

## Security Error Codes

### Authentication Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SEC_AUTHENTICATION_FAILED` | 401 | Authentication failed | Verify credentials |
| `SEC_INVALID_CREDENTIALS` | 401 | Invalid username or password | Use correct credentials |
| `SEC_TOKEN_EXPIRED` | 401 | JWT token expired | Refresh token |
| `SEC_TOKEN_INVALID` | 401 | Invalid or malformed token | Obtain new token |
| `SEC_TOKEN_REVOKED` | 401 | Token has been revoked | Login again |
| `SEC_ACCOUNT_LOCKED` | 403 | Account locked due to failed attempts | Contact administrator |
| `SEC_ACCOUNT_DISABLED` | 403 | Account disabled | Contact administrator |

### Authorization Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SEC_PERMISSION_DENIED` | 403 | Insufficient permissions | Request access or use different account |
| `SEC_ROLE_NOT_FOUND` | 404 | Role does not exist | Verify role name |
| `SEC_UNAUTHORIZED_ACCESS` | 403 | Unauthorized access attempt | Check permissions |
| `SEC_RBAC_VIOLATION` | 403 | RBAC policy violation | Contact administrator |

### Encryption Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SEC_ENCRYPTION_FAILED` | 500 | Data encryption failed | Check encryption configuration |
| `SEC_DECRYPTION_FAILED` | 500 | Data decryption failed | Verify encryption key |
| `SEC_KEY_NOT_FOUND` | 404 | Encryption key not found | Restore or generate key |
| `SEC_KEY_EXPIRED` | 403 | Encryption key expired | Rotate key |

### Audit Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `SEC_AUDIT_LOG_FULL` | 507 | Audit log storage full | Archive logs |
| `SEC_AUDIT_WRITE_FAILED` | 500 | Failed to write audit log | Check storage |

---

## Network Error Codes

### Connection Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `NET_CONNECTION_FAILED` | 503 | Connection failed | Check network connectivity |
| `NET_CONNECTION_TIMEOUT` | 504 | Connection timeout | Retry or check server status |
| `NET_CONNECTION_REFUSED` | 503 | Connection refused | Verify server is running |
| `NET_CONNECTION_RESET` | 503 | Connection reset by peer | Retry connection |
| `NET_TOO_MANY_CONNECTIONS` | 503 | Maximum connections exceeded | Wait or increase limit |
| `NET_CONNECTION_CLOSED` | 503 | Connection unexpectedly closed | Reconnect |

### Protocol Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `NET_PROTOCOL_ERROR` | 400 | Protocol error | Check client implementation |
| `NET_INVALID_MESSAGE` | 400 | Invalid message format | Fix message format |
| `NET_MESSAGE_TOO_LARGE` | 413 | Message exceeds size limit | Reduce message size |
| `NET_UNSUPPORTED_VERSION` | 400 | Unsupported protocol version | Upgrade client |

### Rate Limiting

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `NET_RATE_LIMIT_EXCEEDED` | 429 | Rate limit exceeded | Wait before retrying |
| `NET_BANDWIDTH_EXCEEDED` | 429 | Bandwidth limit exceeded | Reduce request rate |

---

## Cluster Error Codes

### Node Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `CLU_NODE_NOT_FOUND` | 404 | Cluster node not found | Verify node ID |
| `CLU_NODE_OFFLINE` | 503 | Node is offline | Check node status |
| `CLU_NODE_UNREACHABLE` | 503 | Cannot reach node | Check network |
| `CLU_NODE_FAILURE` | 500 | Node failure detected | Investigate node failure |

### Replication Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `CLU_REPLICATION_LAG` | 503 | Replication lag too high | Check replication status |
| `CLU_REPLICATION_FAILED` | 500 | Replication failed | Check logs |
| `CLU_STANDBY_NOT_READY` | 503 | Standby not ready | Wait for standby initialization |
| `CLU_PRIMARY_NOT_FOUND` | 404 | Primary node not found | Check cluster configuration |

### Consensus Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `CLU_NO_QUORUM` | 503 | Cluster quorum not available | Check node availability |
| `CLU_SPLIT_BRAIN` | 500 | Split-brain detected | Resolve partition |
| `CLU_LEADER_NOT_FOUND` | 503 | Leader not elected | Wait for leader election |

### Cache Fusion Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `CLU_BLOCK_TRANSFER_FAILED` | 500 | Block transfer failed | Check interconnect |
| `CLU_LOCK_CONFLICT` | 409 | Global lock conflict | Retry operation |
| `CLU_GRD_UNAVAILABLE` | 503 | GRD service unavailable | Check GRD status |

---

## GraphQL Error Codes

### Query Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `GQL_INVALID_QUERY` | 400 | Invalid GraphQL query | Fix query syntax |
| `GQL_PARSE_ERROR` | 400 | Query parse error | Check query structure |
| `GQL_VALIDATION_ERROR` | 400 | Query validation failed | Fix validation errors |
| `GQL_UNKNOWN_FIELD` | 400 | Unknown field in query | Use valid field names |
| `GQL_UNKNOWN_TYPE` | 400 | Unknown type in query | Use valid types |

### Execution Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `GQL_EXECUTION_ERROR` | 500 | Query execution failed | Check query and data |
| `GQL_COMPLEXITY_EXCEEDED` | 400 | Query complexity too high | Simplify query |
| `GQL_DEPTH_EXCEEDED` | 400 | Query depth too deep | Reduce nesting |
| `GQL_TIMEOUT` | 408 | Query timeout | Optimize query |

### Subscription Errors

| Code | HTTP | Description | Resolution |
|------|------|-------------|------------|
| `GQL_SUBSCRIPTION_FAILED` | 500 | Subscription failed | Retry subscription |
| `GQL_WEBSOCKET_ERROR` | 500 | WebSocket error | Check connection |

---

## PL/SQL Error Codes

### Runtime Errors

| Code | SQL Code | Description | Resolution |
|------|----------|-------------|------------|
| `PLSQL_NO_DATA_FOUND` | -1403 | SELECT INTO returned no rows | Add exception handler or check data |
| `PLSQL_TOO_MANY_ROWS` | -1422 | SELECT INTO returned multiple rows | Add LIMIT or use cursor |
| `PLSQL_ZERO_DIVIDE` | -1476 | Division by zero | Add zero check |
| `PLSQL_VALUE_ERROR` | -6502 | Type conversion or constraint error | Validate data types |
| `PLSQL_INVALID_CURSOR` | -1001 | Invalid cursor operation | Open cursor before fetch |
| `PLSQL_DUP_VAL_ON_INDEX` | -1 | Duplicate key on unique index | Use unique value |
| `PLSQL_INVALID_NUMBER` | -1722 | Invalid number | Validate numeric input |
| `PLSQL_PROGRAM_ERROR` | -6501 | Internal PL/SQL error | Check program logic |

### User-Defined Errors

| Code Range | Description | Usage |
|------------|-------------|-------|
| `-20000` to `-20999` | User-defined application errors | `RAISE_APPLICATION_ERROR(-20001, 'Custom error')` |

---

## Error Handling Best Practices

### 1. Catch and Handle Errors

```sql
-- SQL
BEGIN
    -- Your code
EXCEPTION
    WHEN OTHERS THEN
        ROLLBACK;
        RAISE;
END;
```

```javascript
// JavaScript
try {
    const result = await db.query('SELECT * FROM users');
} catch (error) {
    if (error.code === 'DB_TABLE_NOT_FOUND') {
        // Handle missing table
    } else if (error.code === 'TXN_DEADLOCK_DETECTED') {
        // Retry transaction
    } else {
        // Log and propagate
        console.error('Database error:', error);
        throw error;
    }
}
```

### 2. Retry Logic for Transient Errors

```javascript
async function retryableQuery(query, maxRetries = 3) {
    const retryableCodes = [
        'TXN_DEADLOCK_DETECTED',
        'TXN_SERIALIZATION_FAILURE',
        'NET_CONNECTION_TIMEOUT'
    ];

    for (let i = 0; i < maxRetries; i++) {
        try {
            return await db.query(query);
        } catch (error) {
            if (retryableCodes.includes(error.code) && i < maxRetries - 1) {
                await sleep(Math.pow(2, i) * 1000);  // Exponential backoff
                continue;
            }
            throw error;
        }
    }
}
```

### 3. User-Friendly Error Messages

```javascript
function getUserMessage(error) {
    const messages = {
        'DB_TABLE_NOT_FOUND': 'The requested data could not be found.',
        'SEC_PERMISSION_DENIED': 'You do not have permission to perform this action.',
        'TXN_DEADLOCK_DETECTED': 'Your request conflicted with another operation. Please try again.',
        'NET_RATE_LIMIT_EXCEEDED': 'Too many requests. Please wait a moment and try again.'
    };

    return messages[error.code] || 'An unexpected error occurred. Please contact support.';
}
```

### 4. Log Errors with Context

```javascript
logger.error('Database operation failed', {
    error_code: error.code,
    error_message: error.message,
    request_id: requestId,
    user_id: userId,
    operation: 'query_users',
    query: sanitizeQuery(query),
    stack_trace: error.stack
});
```

---

## Troubleshooting Guide

### Common Error Patterns

| Error Pattern | Likely Cause | Solution |
|--------------|--------------|----------|
| Frequent `TXN_DEADLOCK_DETECTED` | Lock contention | Optimize transaction order, reduce transaction scope |
| `DB_FULL` errors | Disk space exhausted | Free up space, archive old data |
| `NET_TOO_MANY_CONNECTIONS` | Connection leak | Implement connection pooling, close connections |
| `TXN_TIMEOUT` | Long-running transactions | Optimize queries, break into smaller transactions |
| `SEC_TOKEN_EXPIRED` frequently | Short token lifetime | Implement token refresh logic |

---

## Support and Escalation

### Error Severity Levels

| Level | Examples | Action |
|-------|----------|--------|
| **Critical** | `DB_CORRUPTED`, `CLU_SPLIT_BRAIN` | Immediate escalation |
| **High** | `DB_FULL`, `CLU_NO_QUORUM` | Escalate within 1 hour |
| **Medium** | `TXN_DEADLOCK_DETECTED` (frequent) | Investigate and optimize |
| **Low** | `SQL_PARSE_ERROR`, `SEC_PERMISSION_DENIED` | User education |

### When to Contact Support

Contact support when encountering:
- 500-level HTTP errors (server errors)
- `DB_CORRUPTED` or data integrity issues
- Cluster errors affecting availability
- Repeated deadlocks or performance degradation
- Security incidents

---

**Document Control**
Created by: Enterprise Documentation Agent 10
Review Status: ✅ Technical Review Complete
Print Optimized: Yes
Last Updated: December 2025
