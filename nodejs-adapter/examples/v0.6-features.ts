/**
 * RustyDB Node.js Adapter v0.6.0 - New Features Examples
 *
 * This example demonstrates all the new features added in version 0.6.0:
 * - Native N-API bindings
 * - Prepared statements
 * - Result streaming
 * - Enhanced connection pooling
 */

import {
  createRustyDbClient,
  createConfig,
  // Native bindings
  initializeNativeBindings,
  getNativeBindings,
  // Prepared statements
  PreparedStatementManager,
  // Streaming
  QueryResultStream,
  StreamManager,
  streamQuery,
  // Connection pooling
  ConnectionPool,
  BaseClient,
} from '@rustydb/adapter';

// ============================================================================
// Example 1: Native Bindings (N-API)
// ============================================================================

async function example1_NativeBindings() {
  console.log('\n=== Example 1: Native Bindings ===\n');

  // Initialize native bindings (will fall back to HTTP if not available)
  const nativeAvailable = await initializeNativeBindings({
    enabled: true,
    poolSize: 5,
  });

  console.log(`Native bindings available: ${nativeAvailable}`);

  const bindings = getNativeBindings();

  if (bindings.isNativeAvailable()) {
    console.log('Using native N-API bindings for optimal performance');

    // Native operations would be significantly faster
    // Currently falls back to HTTP since native module is not yet compiled
  } else {
    console.log('Using HTTP fallback (native bindings not compiled)');
  }
}

// ============================================================================
// Example 2: Prepared Statements
// ============================================================================

async function example2_PreparedStatements() {
  console.log('\n=== Example 2: Prepared Statements ===\n');

  const config = createConfig()
    .server({ host: 'localhost', port: 5432 })
    .api({ baseUrl: 'http://localhost:8080' })
    .build();

  const client = createRustyDbClient(config);

  // Create a prepared statement manager
  const stmtManager = new PreparedStatementManager(
    client.getHttpClient() as any,
    100 // max cache size
  );

  try {
    // Prepare a statement (cached for reuse)
    console.log('Preparing statement...');
    const selectStmt = await stmtManager.prepare(
      'SELECT * FROM users WHERE email = $1 AND active = $2'
    );

    console.log(`Prepared statement ID: ${selectStmt.getId()}`);
    console.log(`Parameter count: ${selectStmt.getParamCount()}`);

    // Execute with different parameters (statement is reused)
    console.log('\nExecuting with parameters...');
    const result1 = await selectStmt.execute(['user1@example.com', true]);
    console.log(`Found ${result1.rows.length} users for user1@example.com`);

    const result2 = await selectStmt.execute(['user2@example.com', true]);
    console.log(`Found ${result2.rows.length} users for user2@example.com`);

    // Get metadata
    const metadata = await selectStmt.getMetadata();
    console.log(`\nStatement metadata:`);
    console.log(`  Executions: ${metadata.executionCount}`);
    console.log(`  Avg time: ${metadata.avgExecutionTimeMs.toFixed(2)}ms`);

    // Stream results from prepared statement
    console.log('\nStreaming results...');
    let count = 0;
    for await (const row of selectStmt.executeStream(['active@example.com', true])) {
      count++;
      if (count <= 5) {
        console.log(`  Row ${count}:`, row);
      }
    }
    console.log(`Total rows streamed: ${count}`);

    // Clean up
    await selectStmt.close();
    await stmtManager.close();
  } catch (error) {
    console.error('Prepared statement error:', error);
  }
}

// ============================================================================
// Example 3: Result Streaming
// ============================================================================

async function example3_ResultStreaming() {
  console.log('\n=== Example 3: Result Streaming ===\n');

  const config = createConfig()
    .api({ baseUrl: 'http://localhost:8080' })
    .build();

  const client = createRustyDbClient(config);
  const streamManager = new StreamManager(client.getHttpClient() as any, 5);

  try {
    // Example 3a: Event-based streaming
    console.log('Event-based streaming:');

    const stream = streamManager.createStream<{ id: number; name: string; email: string }>(
      'SELECT * FROM users ORDER BY created_at DESC',
      [],
      {
        batchSize: 100,
        maxRows: 10000,
      }
    );

    let rowCount = 0;

    stream.on('data', (row) => {
      rowCount++;
      if (rowCount <= 3) {
        console.log(`  Row ${rowCount}:`, row);
      }
    });

    stream.on('batch', (rows, batchNum) => {
      console.log(`  Batch ${batchNum}: ${rows.length} rows`);
    });

    stream.on('stats', (stats) => {
      console.log(`  Stats: ${stats.rowsStreamed} rows, ${stats.rowsPerSecond.toFixed(0)} rows/s`);
    });

    stream.on('end', () => {
      console.log(`  Streaming complete: ${rowCount} total rows`);
    });

    stream.on('error', (error) => {
      console.error('  Stream error:', error);
    });

    stream.start();

    // Wait for stream to complete
    await new Promise((resolve) => stream.on('end', resolve));

    // Example 3b: Async iterator streaming
    console.log('\nAsync iterator streaming:');

    let iterCount = 0;
    for await (const row of streamQuery(
      client.getHttpClient() as any,
      'SELECT * FROM products WHERE category = $1',
      ['electronics'],
      { batchSize: 50 }
    )) {
      iterCount++;
      if (iterCount <= 3) {
        console.log(`  Row ${iterCount}:`, row);
      }
    }
    console.log(`  Total: ${iterCount} rows`);

    streamManager.destroyAll();
  } catch (error) {
    console.error('Streaming error:', error);
  }
}

// ============================================================================
// Example 4: Connection Pooling
// ============================================================================

async function example4_ConnectionPooling() {
  console.log('\n=== Example 4: Enhanced Connection Pooling ===\n');

  // Define connection factory
  const createConnection = async (): Promise<BaseClient> => {
    return new BaseClient('http://localhost:8080', {
      timeout: 30000,
      headers: {
        'User-Agent': 'RustyDB-Adapter/0.6.0',
      },
    });
  };

  // Create connection pool
  const pool = new ConnectionPool(createConnection, {
    minConnections: 2,
    maxConnections: 10,
    acquireTimeout: 5000,
    idleTimeout: 60000,
    validateOnAcquire: true,
    healthCheckInterval: 30000,
    logging: true,
  });

  // Initialize pool
  await pool.initialize();

  try {
    // Get pool statistics
    console.log('Initial pool stats:', pool.getStats());

    // Example 4a: Manual acquire/release
    console.log('\nManual connection management:');
    const conn1 = await pool.acquire();
    console.log(`Acquired connection: ${conn1.id}`);
    console.log(`Connection state: ${conn1.state}`);
    console.log(`Usage count: ${conn1.usageCount}`);

    // Use the connection
    await conn1.client.get('/api/v1/health');

    await pool.release(conn1);
    console.log('Connection released');

    // Example 4b: Auto-managed with withConnection
    console.log('\nAuto-managed connection:');
    const result = await pool.withConnection(async (connection) => {
      console.log(`Using connection ${connection.id}`);
      return connection.client.get('/api/v1/health');
    });
    console.log('Query result:', result);

    // Example 4c: Concurrent connections
    console.log('\nConcurrent connections:');
    const promises = [];
    for (let i = 0; i < 5; i++) {
      promises.push(
        pool.withConnection(async (conn) => {
          console.log(`  Worker ${i} using connection ${conn.id}`);
          await new Promise((resolve) => setTimeout(resolve, 100));
          return i;
        })
      );
    }

    const results = await Promise.all(promises);
    console.log('All workers completed:', results);

    // Get final statistics
    const finalStats = pool.getStats();
    console.log('\nFinal pool stats:');
    console.log(`  Total connections: ${finalStats.totalConnections}`);
    console.log(`  Active: ${finalStats.activeConnections}`);
    console.log(`  Idle: ${finalStats.idleConnections}`);
    console.log(`  Total acquired: ${finalStats.totalAcquired}`);
    console.log(`  Avg acquire time: ${finalStats.avgAcquireTimeMs.toFixed(2)}ms`);

    // Example 4d: Connection lifecycle events
    console.log('\nListening to pool events:');
    pool.on('acquire', (conn) => {
      console.log(`  [Event] Connection acquired: ${conn.id}`);
    });
    pool.on('release', (conn) => {
      console.log(`  [Event] Connection released: ${conn.id}`);
    });
    pool.on('create', (conn) => {
      console.log(`  [Event] Connection created: ${conn.id}`);
    });
    pool.on('destroy', (conn) => {
      console.log(`  [Event] Connection destroyed: ${conn.id}`);
    });

    // Trigger some events
    const testConn = await pool.acquire();
    await pool.release(testConn);
  } finally {
    // Clean up
    await pool.close();
    console.log('\nPool closed');
  }
}

// ============================================================================
// Example 5: Combined Usage
// ============================================================================

async function example5_CombinedUsage() {
  console.log('\n=== Example 5: Combined Features ===\n');

  const config = createConfig()
    .api({ baseUrl: 'http://localhost:8080' })
    .build();

  const client = createRustyDbClient(config);

  // Create connection pool
  const pool = new ConnectionPool(
    async () => new BaseClient('http://localhost:8080'),
    { minConnections: 2, maxConnections: 5, logging: false }
  );
  await pool.initialize();

  // Create prepared statement manager
  const stmtManager = new PreparedStatementManager(client.getHttpClient() as any);

  // Create stream manager
  const streamManager = new StreamManager(client.getHttpClient() as any);

  try {
    console.log('Demonstrating combined usage:\n');

    // Use connection pool with prepared statements
    await pool.withConnection(async (pooledConn) => {
      console.log('1. Acquired connection from pool');

      // Prepare and execute statement
      const stmt = await stmtManager.prepare('SELECT * FROM users WHERE age > $1');
      console.log('2. Prepared statement cached');

      const result = await stmt.execute([18], { maxRows: 10 });
      console.log(`3. Executed query, found ${result.rows.length} rows`);

      // Stream large results
      console.log('4. Streaming large dataset...');
      const stream = streamManager.createStream(
        'SELECT * FROM logs WHERE created_at > $1',
        [new Date('2024-01-01')],
        { batchSize: 1000, maxRows: 100000 }
      );

      let streamCount = 0;
      stream.on('data', () => streamCount++);
      stream.on('end', () => {
        console.log(`5. Streamed ${streamCount} log entries`);
      });

      stream.start();
      await new Promise((resolve) => stream.on('end', resolve));
    });

    console.log('\nAll operations completed successfully!');
  } finally {
    // Clean up all resources
    await stmtManager.close();
    streamManager.destroyAll();
    await pool.close();
    console.log('All resources cleaned up');
  }
}

// ============================================================================
// Run All Examples
// ============================================================================

async function runAllExamples() {
  console.log('╔═══════════════════════════════════════════════════════════╗');
  console.log('║   RustyDB Node.js Adapter v0.6.0 - New Features Demo     ║');
  console.log('╚═══════════════════════════════════════════════════════════╝');

  try {
    await example1_NativeBindings();
    await example2_PreparedStatements();
    await example3_ResultStreaming();
    await example4_ConnectionPooling();
    await example5_CombinedUsage();

    console.log('\n✅ All examples completed successfully!\n');
  } catch (error) {
    console.error('\n❌ Example failed:', error);
    process.exit(1);
  }
}

// Run examples if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllExamples();
}

export {
  example1_NativeBindings,
  example2_PreparedStatements,
  example3_ResultStreaming,
  example4_ConnectionPooling,
  example5_CombinedUsage,
};
