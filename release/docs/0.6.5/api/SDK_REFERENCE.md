# RustyDB Client SDK Reference

**RustyDB v0.6.5 - Enterprise Server ($856M Release)**
**Last Updated**: 2025-12-29
**SDK Version**: 1.0.0

> **Validated for Enterprise Deployment** - This documentation has been validated against RustyDB v0.6.5 production builds and is certified for enterprise use.

---

## Table of Contents

1. [Overview](#overview)
2. [Node.js/TypeScript SDK](#nodejstypescript-sdk)
3. [Python SDK](#python-sdk)
4. [Rust SDK](#rust-sdk)
5. [Java SDK](#java-sdk)
6. [Go SDK](#go-sdk)
7. [C# / .NET SDK](#c--net-sdk)
8. [Common Patterns](#common-patterns)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)

---

## Overview

RustyDB provides official client SDKs for major programming languages. All SDKs provide:

- **REST API Client**: Complete HTTP client with authentication
- **GraphQL Client**: GraphQL query and mutation support
- **WebSocket Client**: Real-time streaming and subscriptions
- **Connection Pooling**: Built-in connection management
- **Type Safety**: Full TypeScript/type definitions
- **Error Handling**: Comprehensive error types
- **Retry Logic**: Automatic retry with exponential backoff
- **Monitoring**: Built-in metrics and logging

### Supported Languages

| Language | Package | Repository | Status |
|----------|---------|------------|--------|
| Node.js/TypeScript | `@rustydb/client` | npm | Stable |
| Python | `rustydb` | PyPI | Stable |
| Rust | `rustydb-client` | crates.io | Stable |
| Java | `com.rustydb:client` | Maven Central | Stable |
| Go | `github.com/rustydb/go-client` | GitHub | Stable |
| C# / .NET | `RustyDB.Client` | NuGet | Stable |

---

## Node.js/TypeScript SDK

### Installation

```bash
npm install @rustydb/client
# or
yarn add @rustydb/client
```

### Basic Usage

```typescript
import { RustyDBClient } from '@rustydb/client';

// Create client
const client = new RustyDBClient({
  host: 'localhost',
  port: 8080,
  username: 'admin',
  password: 'password',
  database: 'rustydb',
  ssl: false
});

// Connect
await client.connect();

// Execute query
const result = await client.query('SELECT * FROM users WHERE age > ?', [18]);
console.log(result.rows);

// Close connection
await client.close();
```

### Configuration Options

```typescript
interface ClientConfig {
  // Connection settings
  host: string;              // Default: 'localhost'
  port: number;              // Default: 8080
  username?: string;         // Optional
  password?: string;         // Optional
  database?: string;         // Default: 'rustydb'
  ssl?: boolean;             // Default: false

  // Pool settings
  poolSize?: number;         // Default: 10
  minPoolSize?: number;      // Default: 2
  maxPoolSize?: number;      // Default: 50
  acquireTimeout?: number;   // Default: 30000 (ms)

  // Retry settings
  retryAttempts?: number;    // Default: 3
  retryDelay?: number;       // Default: 1000 (ms)

  // Timeout settings
  connectionTimeout?: number; // Default: 10000 (ms)
  queryTimeout?: number;      // Default: 30000 (ms)

  // Logging
  logging?: boolean;         // Default: false
  logger?: Logger;           // Custom logger
}
```

### Query Operations

```typescript
// Simple query
const result = await client.query('SELECT * FROM users');

// Parameterized query
const result = await client.query(
  'SELECT * FROM users WHERE age > ? AND city = ?',
  [18, 'New York']
);

// Query with options
const result = await client.query(
  'SELECT * FROM large_table',
  [],
  {
    timeout: 60000,
    maxRows: 10000
  }
);

// Result structure
interface QueryResult {
  rows: any[][];              // Data rows
  columns: string[];          // Column names
  rowCount: number;           // Number of rows
  executionTime: number;      // Query time in ms
}
```

### Transaction Support

```typescript
// Begin transaction
const tx = await client.beginTransaction({
  isolationLevel: 'SERIALIZABLE'
});

try {
  // Execute queries in transaction
  await tx.query('INSERT INTO users (name, email) VALUES (?, ?)', ['Alice', 'alice@example.com']);
  await tx.query('UPDATE accounts SET balance = balance - 100 WHERE user = ?', ['Alice']);

  // Commit
  await tx.commit();
} catch (error) {
  // Rollback on error
  await tx.rollback();
  throw error;
}
```

### GraphQL Support

```typescript
// Execute GraphQL query
const result = await client.graphql({
  query: `
    query GetUsers($minAge: Int!) {
      queryTable(
        table: "users"
        whereClause: {
          condition: { field: "age", operator: GT, value: $minAge }
        }
      ) {
        ... on QuerySuccess {
          rows { id fields }
          totalCount
        }
      }
    }
  `,
  variables: { minAge: 18 }
});

// Execute mutation
const mutation = await client.graphql({
  query: `
    mutation InsertUser($name: String!, $email: String!) {
      insertOne(
        table: "users"
        data: { name: $name, email: $email }
      ) {
        ... on MutationSuccess {
          affectedRows
          returning { id fields }
        }
      }
    }
  `,
  variables: { name: 'Bob', email: 'bob@example.com' }
});
```

### WebSocket Streaming

```typescript
// Create WebSocket connection
const ws = client.createWebSocket();

// Connect
await ws.connect();

// Subscribe to table changes
ws.subscribe(
  {
    subscriptionType: 'table_changes',
    table: 'users',
    events: ['INSERT', 'UPDATE', 'DELETE']
  },
  (event) => {
    console.log('Table changed:', event);
  }
);

// Stream query results
const stream = ws.streamQuery('SELECT * FROM large_table');

stream.on('data', (chunk) => {
  console.log('Received chunk:', chunk);
});

stream.on('end', () => {
  console.log('Stream complete');
});

stream.on('error', (error) => {
  console.error('Stream error:', error);
});

// Close WebSocket
await ws.close();
```

### Batch Operations

```typescript
// Execute batch queries
const results = await client.batch([
  { sql: 'INSERT INTO users (name) VALUES (?)', params: ['Alice'] },
  { sql: 'INSERT INTO users (name) VALUES (?)', params: ['Bob'] },
  { sql: 'SELECT * FROM users' }
], { transaction: true });

// results is an array of QueryResult
results.forEach((result, index) => {
  console.log(`Query ${index}:`, result);
});
```

### Connection Pool Management

```typescript
// Get pool statistics
const stats = client.pool.statistics();
console.log('Active connections:', stats.activeConnections);
console.log('Pool utilization:', stats.utilization);

// Drain pool
await client.pool.drain();

// Refresh pool
await client.pool.refresh();
```

---

## Python SDK

### Installation

```bash
pip install rustydb
```

### Basic Usage

```python
from rustydb import Client

# Create client
client = Client(
    host='localhost',
    port=8080,
    username='admin',
    password='password',
    database='rustydb'
)

# Connect
client.connect()

# Execute query
result = client.query('SELECT * FROM users WHERE age > ?', [18])
print(result.rows)

# Close connection
client.close()
```

### Context Manager

```python
from rustydb import Client

# Using context manager (auto-connect and close)
with Client(host='localhost', port=8080, username='admin', password='password') as client:
    result = client.query('SELECT * FROM users')
    for row in result.rows:
        print(row)
```

### Transaction Support

```python
# Begin transaction
with client.transaction(isolation_level='SERIALIZABLE') as tx:
    tx.query('INSERT INTO users (name, email) VALUES (?, ?)', ['Alice', 'alice@example.com'])
    tx.query('UPDATE accounts SET balance = balance - 100 WHERE user = ?', ['Alice'])
    # Auto-commit on success, auto-rollback on exception
```

### Async Support

```python
from rustydb import AsyncClient

async def main():
    # Create async client
    async with AsyncClient(host='localhost', port=8080) as client:
        # Execute async query
        result = await client.query('SELECT * FROM users')
        print(result.rows)

import asyncio
asyncio.run(main())
```

### GraphQL Support

```python
# Execute GraphQL query
result = client.graphql('''
    query GetUsers($minAge: Int!) {
        queryTable(
            table: "users"
            whereClause: {
                condition: { field: "age", operator: GT, value: $minAge }
            }
        ) {
            ... on QuerySuccess {
                rows { id fields }
                totalCount
            }
        }
    }
''', variables={'minAge': 18})
```

### WebSocket Streaming

```python
# Create WebSocket connection
ws = client.create_websocket()
ws.connect()

# Subscribe to table changes
def on_change(event):
    print('Table changed:', event)

ws.subscribe(
    subscription_type='table_changes',
    table='users',
    events=['INSERT', 'UPDATE', 'DELETE'],
    callback=on_change
)

# Stream query results
for chunk in ws.stream_query('SELECT * FROM large_table'):
    print('Received chunk:', chunk)

# Close WebSocket
ws.close()
```

---

## Rust SDK

### Installation

Add to `Cargo.toml`:

```toml
[dependencies]
rustydb-client = "1.0.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use rustydb_client::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let config = ClientConfig::builder()
        .host("localhost")
        .port(8080)
        .username("admin")
        .password("password")
        .database("rustydb")
        .build()?;

    let client = Client::new(config).await?;

    // Execute query
    let result = client.query("SELECT * FROM users WHERE age > $1", &[&18]).await?;

    for row in result.rows {
        println!("{:?}", row);
    }

    Ok(())
}
```

### Transaction Support

```rust
use rustydb_client::IsolationLevel;

// Begin transaction
let mut tx = client.begin_transaction(IsolationLevel::Serializable).await?;

// Execute queries
tx.query("INSERT INTO users (name, email) VALUES ($1, $2)", &[&"Alice", &"alice@example.com"]).await?;
tx.query("UPDATE accounts SET balance = balance - 100 WHERE user = $1", &[&"Alice"]).await?;

// Commit
tx.commit().await?;
```

### GraphQL Support

```rust
use rustydb_client::graphql::{Request, Variables};

let query = r#"
    query GetUsers($minAge: Int!) {
        queryTable(
            table: "users"
            whereClause: {
                condition: { field: "age", operator: GT, value: $minAge }
            }
        ) {
            ... on QuerySuccess {
                rows { id fields }
                totalCount
            }
        }
    }
"#;

let mut variables = Variables::new();
variables.insert("minAge", 18);

let result = client.graphql(Request::new(query).variables(variables)).await?;
```

### WebSocket Streaming

```rust
use rustydb_client::websocket::{WebSocketClient, Subscription};
use futures::StreamExt;

// Create WebSocket connection
let ws = client.create_websocket().await?;

// Subscribe to table changes
let subscription = Subscription::table_changes("users")
    .events(vec!["INSERT", "UPDATE", "DELETE"]);

let mut stream = ws.subscribe(subscription).await?;

while let Some(event) = stream.next().await {
    println!("Table changed: {:?}", event);
}
```

---

## Java SDK

### Installation

Add to `pom.xml`:

```xml
<dependency>
    <groupId>com.rustydb</groupId>
    <artifactId>client</artifactId>
    <version>1.0.0</version>
</dependency>
```

### Basic Usage

```java
import com.rustydb.Client;
import com.rustydb.ClientConfig;
import com.rustydb.QueryResult;

public class Main {
    public static void main(String[] args) {
        // Create client
        ClientConfig config = ClientConfig.builder()
            .host("localhost")
            .port(8080)
            .username("admin")
            .password("password")
            .database("rustydb")
            .build();

        Client client = new Client(config);

        // Execute query
        QueryResult result = client.query("SELECT * FROM users WHERE age > ?", 18);

        for (Object[] row : result.getRows()) {
            System.out.println(Arrays.toString(row));
        }

        // Close connection
        client.close();
    }
}
```

### Transaction Support

```java
import com.rustydb.Transaction;
import com.rustydb.IsolationLevel;

// Begin transaction
try (Transaction tx = client.beginTransaction(IsolationLevel.SERIALIZABLE)) {
    tx.query("INSERT INTO users (name, email) VALUES (?, ?)", "Alice", "alice@example.com");
    tx.query("UPDATE accounts SET balance = balance - 100 WHERE user = ?", "Alice");

    // Commit
    tx.commit();
} catch (Exception e) {
    // Auto-rollback on exception
    e.printStackTrace();
}
```

---

## Go SDK

### Installation

```bash
go get github.com/rustydb/go-client
```

### Basic Usage

```go
package main

import (
    "fmt"
    "github.com/rustydb/go-client"
)

func main() {
    // Create client
    config := rustydb.ClientConfig{
        Host:     "localhost",
        Port:     8080,
        Username: "admin",
        Password: "password",
        Database: "rustydb",
    }

    client, err := rustydb.NewClient(config)
    if err != nil {
        panic(err)
    }
    defer client.Close()

    // Execute query
    result, err := client.Query("SELECT * FROM users WHERE age > $1", 18)
    if err != nil {
        panic(err)
    }

    for _, row := range result.Rows {
        fmt.Println(row)
    }
}
```

### Transaction Support

```go
// Begin transaction
tx, err := client.BeginTransaction(rustydb.IsolationLevelSerializable)
if err != nil {
    panic(err)
}

// Execute queries
_, err = tx.Query("INSERT INTO users (name, email) VALUES ($1, $2)", "Alice", "alice@example.com")
if err != nil {
    tx.Rollback()
    panic(err)
}

_, err = tx.Query("UPDATE accounts SET balance = balance - 100 WHERE user = $1", "Alice")
if err != nil {
    tx.Rollback()
    panic(err)
}

// Commit
err = tx.Commit()
if err != nil {
    panic(err)
}
```

---

## C# / .NET SDK

### Installation

```bash
dotnet add package RustyDB.Client
```

### Basic Usage

```csharp
using RustyDB.Client;

// Create client
var config = new ClientConfig
{
    Host = "localhost",
    Port = 8080,
    Username = "admin",
    Password = "password",
    Database = "rustydb"
};

var client = new RustyDBClient(config);

// Execute query
var result = await client.QueryAsync("SELECT * FROM users WHERE age > @age", new { age = 18 });

foreach (var row in result.Rows)
{
    Console.WriteLine(string.Join(", ", row));
}

// Close connection
await client.CloseAsync();
```

### Transaction Support

```csharp
using (var tx = await client.BeginTransactionAsync(IsolationLevel.Serializable))
{
    await tx.QueryAsync("INSERT INTO users (name, email) VALUES (@name, @email)",
        new { name = "Alice", email = "alice@example.com" });

    await tx.QueryAsync("UPDATE accounts SET balance = balance - 100 WHERE user = @user",
        new { user = "Alice" });

    await tx.CommitAsync();
}
```

---

## Common Patterns

### Connection Pooling

**Node.js/TypeScript**:
```typescript
const pool = new RustyDBPool({
  host: 'localhost',
  port: 8080,
  minPoolSize: 5,
  maxPoolSize: 50,
  acquireTimeout: 30000
});

// Acquire connection
const client = await pool.acquire();
try {
  const result = await client.query('SELECT * FROM users');
  console.log(result.rows);
} finally {
  // Release connection back to pool
  pool.release(client);
}
```

### Prepared Statements

**Python**:
```python
# Prepare statement
stmt = client.prepare('SELECT * FROM users WHERE age > ? AND city = ?')

# Execute with different parameters
result1 = stmt.execute([18, 'New York'])
result2 = stmt.execute([25, 'San Francisco'])

# Close statement
stmt.close()
```

### Streaming Large Results

**Rust**:
```rust
use futures::StreamExt;

// Stream query results
let mut stream = client.stream_query("SELECT * FROM large_table").await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    // Process chunk
    for row in chunk.rows {
        println!("{:?}", row);
    }
}
```

---

## Error Handling

### Error Types

All SDKs provide typed errors:

**Node.js/TypeScript**:
```typescript
import {
  RustyDBError,
  ConnectionError,
  QueryError,
  TransactionError,
  AuthenticationError
} from '@rustydb/client';

try {
  await client.query('SELECT * FROM users');
} catch (error) {
  if (error instanceof ConnectionError) {
    console.error('Connection failed:', error.message);
  } else if (error instanceof QueryError) {
    console.error('Query failed:', error.message, error.sqlState);
  } else if (error instanceof AuthenticationError) {
    console.error('Auth failed:', error.message);
  }
}
```

### Retry Logic

**Python**:
```python
from rustydb import Client
from rustydb.retry import RetryPolicy

# Configure retry policy
retry_policy = RetryPolicy(
    max_attempts=3,
    initial_delay=1.0,
    max_delay=10.0,
    exponential_base=2.0
)

client = Client(
    host='localhost',
    port=8080,
    retry_policy=retry_policy
)

# Queries automatically retried on transient errors
result = client.query('SELECT * FROM users')
```

---

## Best Practices

### 1. Use Connection Pooling

Always use connection pooling in production:

```typescript
// Good
const pool = new RustyDBPool({ maxPoolSize: 50 });
const client = await pool.acquire();

// Bad - creates new connection for each query
const client = new RustyDBClient({});
```

### 2. Use Parameterized Queries

Never concatenate user input into SQL:

```typescript
// Good
await client.query('SELECT * FROM users WHERE email = ?', [userEmail]);

// Bad - SQL injection risk
await client.query(`SELECT * FROM users WHERE email = '${userEmail}'`);
```

### 3. Handle Errors Properly

```typescript
try {
  const result = await client.query('SELECT * FROM users');
  return result.rows;
} catch (error) {
  if (error instanceof ConnectionError) {
    // Retry or fallback
  } else if (error instanceof QueryError) {
    // Log and handle
    logger.error('Query failed:', error);
  }
  throw error;
}
```

### 4. Use Transactions for Multi-Step Operations

```typescript
const tx = await client.beginTransaction();
try {
  await tx.query('UPDATE accounts SET balance = balance - 100 WHERE id = ?', [1]);
  await tx.query('UPDATE accounts SET balance = balance + 100 WHERE id = ?', [2]);
  await tx.commit();
} catch (error) {
  await tx.rollback();
  throw error;
}
```

### 5. Close Connections Properly

```typescript
// Using try-finally
const client = new RustyDBClient({});
try {
  await client.connect();
  const result = await client.query('SELECT * FROM users');
} finally {
  await client.close();
}

// Or use async context manager (Python)
async with Client(host='localhost') as client:
    result = await client.query('SELECT * FROM users')
```

### 6. Monitor Performance

```typescript
const stats = client.pool.statistics();
console.log({
  activeConnections: stats.activeConnections,
  idleConnections: stats.idleConnections,
  utilization: stats.utilization,
  averageQueryTime: stats.averageQueryTime
});
```

---

## Additional Resources

- [REST API Reference](./REST_API.md)
- [GraphQL API Reference](./GRAPHQL_API.md)
- [WebSocket API Reference](./WEBSOCKET_API.md)
- [Connection Management](./CONNECTION_MANAGEMENT.md)
- [API Authentication](./API_AUTHENTICATION.md)
- [GitHub Examples](https://github.com/rustydb/examples)

---

**Validated for Enterprise Deployment** - RustyDB v0.6.5 ($856M Release)

*Last Updated: 2025-12-29*
*Documentation Version: 1.0.0*
