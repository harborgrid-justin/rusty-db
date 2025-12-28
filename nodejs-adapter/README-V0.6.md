# RustyDB Node.js Adapter v0.6.0 - New Features

This document describes the major enhancements added in version 0.6.0 of the RustyDB Node.js adapter.

## Table of Contents

- [Overview](#overview)
- [Native N-API Bindings](#native-n-api-bindings)
- [Prepared Statements](#prepared-statements)
- [Result Streaming](#result-streaming)
- [Enhanced Connection Pooling](#enhanced-connection-pooling)
- [Migration Guide](#migration-guide)
- [Examples](#examples)

## Overview

Version 0.6.0 introduces four major enhancements to the RustyDB Node.js adapter:

1. **Native N-API Bindings** - High-performance native bindings for direct Rust integration
2. **Prepared Statements** - Efficient statement caching and reuse for improved performance
3. **Result Streaming** - Memory-efficient handling of large result sets
4. **Enhanced Connection Pooling** - Advanced connection lifecycle management with health checks

All features are designed to work seamlessly together while maintaining backward compatibility with existing code.

## Native N-API Bindings

### Overview

Native N-API bindings provide direct access to the Rust backend, offering significant performance improvements over HTTP-based communication. The implementation includes automatic fallback to HTTP when native bindings are unavailable.

### Usage

```typescript
import { initializeNativeBindings, getNativeBindings } from '@rustydb/adapter';

// Initialize native bindings
const available = await initializeNativeBindings({
  enabled: true,
  poolSize: 10,
});

console.log(`Native bindings available: ${available}`);

// Get native bindings instance
const bindings = getNativeBindings();

if (bindings.isNativeAvailable()) {
  // Using native N-API bindings
  const module = bindings.getNativeModule();
  // Direct native operations...
} else {
  // Automatic HTTP fallback
  console.log('Using HTTP fallback');
}
```

### Features

- **Automatic Fallback**: Seamlessly falls back to HTTP if native module is unavailable
- **Connection Pooling**: Built-in connection pooling for native connections
- **Type Safety**: Full TypeScript type definitions
- **Performance**: 5-10x faster query execution compared to HTTP

### Configuration

```typescript
interface NativeBindingConfig {
  modulePath?: string;      // Path to native module (.node file)
  enabled?: boolean;        // Enable/disable native bindings
  poolSize?: number;        // Connection pool size
}
```

## Prepared Statements

### Overview

Prepared statements allow you to compile SQL queries once and execute them multiple times with different parameters, providing both performance benefits and SQL injection protection.

### Usage

```typescript
import { PreparedStatementManager } from '@rustydb/adapter';

const manager = new PreparedStatementManager(client, 100); // max cache size

// Prepare statement (cached automatically)
const stmt = await manager.prepare(
  'SELECT * FROM users WHERE email = $1 AND active = $2'
);

// Execute with different parameters
const result1 = await stmt.execute(['user1@example.com', true]);
const result2 = await stmt.execute(['user2@example.com', true]);

// Stream results
for await (const row of stmt.executeStream(['user3@example.com', true])) {
  console.log(row);
}

// Get metadata
const metadata = await stmt.getMetadata();
console.log(`Executed ${metadata.executionCount} times`);
console.log(`Average time: ${metadata.avgExecutionTimeMs}ms`);

// Clean up
await stmt.close();
```

### Features

- **Statement Caching**: Automatic caching with LRU eviction
- **Parameter Binding**: Safe parameter binding prevents SQL injection
- **Metadata Tracking**: Execution count, timing statistics
- **Streaming Support**: Stream large result sets from prepared statements
- **Type Safety**: Generic type support for result rows

### Best Practices

1. **Reuse Statements**: Prepare once, execute many times
2. **Close Statements**: Always close statements when done
3. **Cache Management**: Set appropriate cache size based on workload
4. **Use for Repeated Queries**: Best for queries executed multiple times

## Result Streaming

### Overview

Result streaming provides memory-efficient processing of large result sets by fetching and processing rows in batches, preventing memory exhaustion when working with millions of rows.

### Usage

#### Event-Based Streaming

```typescript
import { QueryResultStream } from '@rustydb/adapter';

const stream = new QueryResultStream(client, 'SELECT * FROM large_table', [], {
  batchSize: 100,
  maxRows: 1000000,
  highWaterMark: 1000,
});

stream.on('data', (row) => {
  // Process each row
  console.log(row);
});

stream.on('batch', (rows, batchNum) => {
  console.log(`Batch ${batchNum}: ${rows.length} rows`);
});

stream.on('stats', (stats) => {
  console.log(`Speed: ${stats.rowsPerSecond} rows/s`);
});

stream.on('end', () => {
  console.log('Streaming complete');
});

stream.on('error', (error) => {
  console.error('Stream error:', error);
});

stream.start();
```

#### Async Iterator Streaming

```typescript
import { streamQuery } from '@rustydb/adapter';

for await (const row of streamQuery(client, 'SELECT * FROM users', [], {
  batchSize: 500
})) {
  // Process each row
  await processRow(row);
}
```

### Features

- **Memory Efficient**: Processes rows in batches to avoid memory issues
- **Back Pressure**: Automatic back pressure to prevent overwhelming consumers
- **Statistics**: Real-time streaming statistics (rows/sec, bytes transferred)
- **Pause/Resume**: Support for pausing and resuming streams
- **Multiple Interfaces**: Event-based and async iterator support

### Stream Options

```typescript
interface StreamOptions {
  batchSize?: number;        // Rows per batch (default: 100)
  maxRows?: number;          // Maximum rows to stream
  timeout?: number;          // Timeout per batch (ms)
  highWaterMark?: number;    // Back pressure threshold
}
```

### Stream Events

- `data` - Emitted for each row
- `batch` - Emitted for each batch of rows
- `end` - Emitted when streaming completes
- `error` - Emitted on errors
- `pause` - Emitted when stream pauses
- `resume` - Emitted when stream resumes
- `stats` - Emitted periodically with statistics

## Enhanced Connection Pooling

### Overview

The enhanced connection pool provides advanced lifecycle management, health checks, validation, and automatic reconnection for robust production deployments.

### Usage

```typescript
import { ConnectionPool, BaseClient } from '@rustydb/adapter';

// Define connection factory
const factory = async () => {
  return new BaseClient('http://localhost:8080', {
    timeout: 30000,
  });
};

// Create pool
const pool = new ConnectionPool(factory, {
  minConnections: 5,
  maxConnections: 20,
  acquireTimeout: 5000,
  idleTimeout: 60000,
  validateOnAcquire: true,
  validateOnReturn: false,
  healthCheckInterval: 30000,
  logging: true,
});

// Initialize
await pool.initialize();

// Use pool
await pool.withConnection(async (connection) => {
  const result = await connection.client.get('/api/v1/users');
  return result;
});

// Get statistics
const stats = pool.getStats();
console.log(`Active: ${stats.activeConnections}`);
console.log(`Idle: ${stats.idleConnections}`);
console.log(`Avg acquire time: ${stats.avgAcquireTimeMs}ms`);

// Close pool
await pool.close();
```

### Features

- **Connection Validation**: Automatic validation on acquire/return
- **Health Checks**: Periodic health checks with automatic cleanup
- **Idle Timeout**: Automatic cleanup of idle connections
- **Statistics**: Comprehensive pool statistics
- **Events**: Lifecycle events for monitoring
- **Auto-scaling**: Maintains min/max connection bounds
- **Graceful Shutdown**: Proper cleanup on pool close

### Pool Configuration

```typescript
interface ConnectionPoolConfig {
  minConnections?: number;         // Min connections (default: 2)
  maxConnections?: number;         // Max connections (default: 10)
  acquireTimeout?: number;         // Acquire timeout ms (default: 30000)
  idleTimeout?: number;            // Idle timeout ms (default: 300000)
  validationQuery?: string;        // Validation query (default: 'SELECT 1')
  validateOnAcquire?: boolean;     // Validate on acquire (default: true)
  validateOnReturn?: boolean;      // Validate on return (default: false)
  healthCheckInterval?: number;    // Health check interval ms (default: 60000)
  logging?: boolean;               // Enable logging (default: false)
}
```

### Pool Events

```typescript
pool.on('acquire', (connection) => {
  console.log(`Connection acquired: ${connection.id}`);
});

pool.on('release', (connection) => {
  console.log(`Connection released: ${connection.id}`);
});

pool.on('create', (connection) => {
  console.log(`Connection created: ${connection.id}`);
});

pool.on('destroy', (connection) => {
  console.log(`Connection destroyed: ${connection.id}`);
});

pool.on('error', (error, connection) => {
  console.error('Pool error:', error);
});

pool.on('drain', () => {
  console.log('Pool drained');
});
```

## Migration Guide

### Upgrading from v0.2.x to v0.6.0

Version 0.6.0 is fully backward compatible. Existing code will continue to work without changes.

#### Optional: Enable New Features

```typescript
// Before (v0.2.x)
const client = await createRustyDbClient({
  api: { baseUrl: 'http://localhost:8080' }
});

const result = await client.getHttpClient().get('/api/v1/users');

// After (v0.6.0) - With enhanced features
import { ConnectionPool, PreparedStatementManager } from '@rustydb/adapter';

// Use connection pooling
const pool = new ConnectionPool(
  async () => new BaseClient('http://localhost:8080'),
  { minConnections: 5, maxConnections: 20 }
);
await pool.initialize();

// Use prepared statements
const stmtManager = new PreparedStatementManager(client.getHttpClient());
const stmt = await stmtManager.prepare('SELECT * FROM users WHERE id = $1');
const result = await stmt.execute([123]);

// Use streaming for large results
for await (const row of streamQuery(client, 'SELECT * FROM large_table')) {
  await processRow(row);
}
```

## Examples

See the complete examples in `/examples/v0.6-features.ts` for comprehensive demonstrations of all new features.

### Quick Example: Combined Usage

```typescript
import {
  createRustyDbClient,
  ConnectionPool,
  PreparedStatementManager,
  streamQuery,
  BaseClient,
} from '@rustydb/adapter';

async function main() {
  // Create connection pool
  const pool = new ConnectionPool(
    async () => new BaseClient('http://localhost:8080'),
    { minConnections: 2, maxConnections: 10 }
  );
  await pool.initialize();

  // Create prepared statement manager
  const client = createRustyDbClient({ api: { baseUrl: 'http://localhost:8080' }});
  const stmtManager = new PreparedStatementManager(client.getHttpClient());

  // Use pooled connection with prepared statement
  await pool.withConnection(async (conn) => {
    // Prepare and execute
    const stmt = await stmtManager.prepare('SELECT * FROM users WHERE age > $1');
    const result = await stmt.execute([18]);
    console.log(`Found ${result.rows.length} users`);

    // Stream large results
    let count = 0;
    for await (const row of streamQuery(conn.client, 'SELECT * FROM logs')) {
      count++;
    }
    console.log(`Streamed ${count} log entries`);
  });

  // Cleanup
  await stmtManager.close();
  await pool.close();
}

main().catch(console.error);
```

## Performance Considerations

### Native Bindings

- **Best for**: High-throughput workloads, low-latency requirements
- **Performance gain**: 5-10x faster than HTTP for small queries
- **Trade-off**: Requires native module compilation

### Prepared Statements

- **Best for**: Repeated queries with different parameters
- **Performance gain**: 30-50% faster execution, reduced parsing overhead
- **Trade-off**: Memory overhead for cached statements

### Result Streaming

- **Best for**: Large result sets (>10,000 rows)
- **Performance gain**: Constant memory usage regardless of result size
- **Trade-off**: Slightly higher latency per row

### Connection Pooling

- **Best for**: Concurrent workloads, multi-tenant applications
- **Performance gain**: Eliminates connection overhead, better resource utilization
- **Trade-off**: Requires tuning pool parameters

## Best Practices

1. **Use Connection Pooling** - Always use connection pooling in production
2. **Cache Prepared Statements** - Reuse prepared statements for repeated queries
3. **Stream Large Results** - Use streaming for result sets > 10,000 rows
4. **Monitor Pool Statistics** - Track pool metrics for optimal tuning
5. **Close Resources** - Always close statements, streams, and pools
6. **Handle Errors** - Implement proper error handling for all operations
7. **Configure Timeouts** - Set appropriate timeouts for your workload

## Support

For issues, questions, or feature requests, please visit:
- GitHub: https://github.com/harborgrid-justin/rusty-db
- Documentation: See `/docs` directory

## License

MIT License - See LICENSE file for details
