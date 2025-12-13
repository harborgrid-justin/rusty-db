/**
 * RustyDB Node.js Adapter - Basic Usage Examples
 * 
 * This file demonstrates common usage patterns for the RustyDB adapter.
 */

import {
  createRustyDbClient,
  initializeRustyDbClient,
  createConfig,
  createStorageClient,
  createTransactionClient,
  createSecurityClient,
  createMonitoringClient,
  IsolationLevel,
} from '../src';

// ============================================================================
// Example 1: Basic Client Usage with Auto-Start
// ============================================================================

async function example1_BasicUsage() {
  console.log('\n=== Example 1: Basic Client Usage ===\n');

  // Create configuration using the builder pattern
  const config = createConfig()
    .server({ host: 'localhost', port: 5432 })
    .api({ baseUrl: 'http://localhost:8080' })
    .autoStart(true)  // Automatically start the server
    .autoStop(true)   // Automatically stop on shutdown
    .startupTimeout(30000)
    .logging({ stdout: true, stderr: true })
    .build();

  // Create and initialize the client
  const client = await initializeRustyDbClient(config);

  try {
    // The server is now running and ready to use
    console.log('Server started successfully!');

    // Get the HTTP client for making API calls
    const http = client.getHttpClient();
    
    // Make a health check
    const health = await http.healthCheck();
    console.log('Health status:', health);

  } finally {
    // Cleanup: shutdown will stop the server automatically
    await client.shutdown();
    console.log('Server stopped successfully!');
  }
}

// ============================================================================
// Example 2: Storage API Usage
// ============================================================================

async function example2_StorageApi() {
  console.log('\n=== Example 2: Storage API ===\n');

  // Create a storage client
  const storage = createStorageClient({
    baseUrl: 'http://localhost:8080',
  });

  try {
    // Get overall storage status
    const status = await storage.getStorageStatus();
    console.log(`Storage utilization: ${status.utilization_percent}%`);
    console.log(`Total space: ${(status.total_space_bytes / 1024 / 1024 / 1024).toFixed(2)} GB`);

    // List all disks
    const disks = await storage.getDisks();
    console.log(`\nFound ${disks.length} disks:`);
    disks.forEach(disk => {
      console.log(`  - ${disk.disk_id}: ${disk.read_iops} IOPS (read)`);
    });

    // Get buffer pool statistics
    const bufferStats = await storage.getBufferPoolStats();
    console.log(`\nBuffer pool hit ratio: ${(bufferStats.hit_ratio * 100).toFixed(2)}%`);
    console.log(`Dirty pages: ${bufferStats.dirty_pages}`);

    // Create a tablespace
    const tablespace = await storage.createTablespace({
      name: 'user_data',
      location: '/data/tablespaces/user_data',
      initial_size_mb: 1024,
      auto_extend: true,
      max_size_mb: 10240,
    });
    console.log(`\nCreated tablespace: ${tablespace.tablespace_id}`);

  } catch (error) {
    console.error('Storage API error:', error);
  }
}

// ============================================================================
// Example 3: Transaction Management
// ============================================================================

async function example3_Transactions() {
  console.log('\n=== Example 3: Transaction Management ===\n');

  const transaction = createTransactionClient({
    baseUrl: 'http://localhost:8080',
  });

  try {
    // Begin a transaction
    const txn = await transaction.beginTransaction({
      isolation_level: IsolationLevel.READ_COMMITTED,
    });
    console.log(`Transaction started: ${txn.transaction_id}`);

    // Create a savepoint
    await transaction.createSavepoint(txn.transaction_id, 'savepoint1');
    console.log('Savepoint created');

    // Perform some operations...
    // (In a real application, you would execute queries here)

    // Commit the transaction
    await transaction.commitTransaction(txn.transaction_id);
    console.log('Transaction committed successfully');

    // Get transaction statistics
    const stats = await transaction.getTransactionStats();
    console.log(`\nTransaction stats:`);
    console.log(`  Active: ${stats.active_count}`);
    console.log(`  Committed: ${stats.committed_count}`);
    console.log(`  Aborted: ${stats.aborted_count}`);

  } catch (error) {
    console.error('Transaction error:', error);
  }
}

// ============================================================================
// Example 4: Security Features
// ============================================================================

async function example4_Security() {
  console.log('\n=== Example 4: Security Features ===\n');

  const security = createSecurityClient({
    baseUrl: 'http://localhost:8080',
  });

  try {
    // Get TDE (Transparent Data Encryption) status
    const tdeStatus = await security.getTdeStatus();
    console.log(`TDE enabled: ${tdeStatus.enabled}`);

    // Create a role
    const role = await security.createRole({
      name: 'app_user',
      description: 'Application user role',
    });
    console.log(`\nCreated role: ${role.role_id}`);

    // Grant permissions
    await security.grantPermission(role.role_id, {
      resource_type: 'table',
      resource_id: 'users',
      permissions: ['SELECT', 'INSERT', 'UPDATE'],
    });
    console.log('Permissions granted');

    // List all roles
    const roles = await security.getRoles();
    console.log(`\nTotal roles: ${roles.length}`);

    // Get audit logs
    const auditLogs = await security.getAuditLogs({
      limit: 10,
    });
    console.log(`\nRecent audit logs: ${auditLogs.length} entries`);

  } catch (error) {
    console.error('Security API error:', error);
  }
}

// ============================================================================
// Example 5: Monitoring and Health Checks
// ============================================================================

async function example5_Monitoring() {
  console.log('\n=== Example 5: Monitoring ===\n');

  const monitoring = createMonitoringClient({
    baseUrl: 'http://localhost:8080',
  });

  try {
    // Health checks
    const liveness = await monitoring.getLivenessProbe();
    console.log(`Liveness: ${liveness.status}`);

    const readiness = await monitoring.getReadinessProbe();
    console.log(`Readiness: ${readiness.status}`);

    // Get system metrics
    const metrics = await monitoring.getMetrics();
    console.log(`\nCollected ${metrics.length} metrics`);

    // Get performance statistics
    const perfStats = await monitoring.getPerformanceStats();
    console.log(`\nPerformance:`);
    console.log(`  CPU: ${perfStats.cpu_usage_percent.toFixed(2)}%`);
    console.log(`  Memory: ${perfStats.memory_usage_percent.toFixed(2)}%`);
    console.log(`  Disk: ${perfStats.disk_usage_percent.toFixed(2)}%`);

    // Get active sessions
    const sessions = await monitoring.getActiveSessions();
    console.log(`\nActive sessions: ${sessions.length}`);

    // Get slow queries
    const slowQueries = await monitoring.getSlowQueries({
      threshold_ms: 1000,
      limit: 10,
    });
    console.log(`Slow queries: ${slowQueries.length}`);

  } catch (error) {
    console.error('Monitoring error:', error);
  }
}

// ============================================================================
// Example 6: Complete Application Example
// ============================================================================

async function example6_CompleteApplication() {
  console.log('\n=== Example 6: Complete Application ===\n');

  // Configuration from environment or defaults
  const config = createConfig()
    .server({ 
      host: process.env.DB_HOST || 'localhost', 
      port: parseInt(process.env.DB_PORT || '5432'),
    })
    .api({ 
      baseUrl: process.env.API_URL || 'http://localhost:8080',
      timeout: 30000,
    })
    .autoStart(true)
    .autoStop(true)
    .logging({ stdout: true, stderr: true })
    .build();

  const client = await initializeRustyDbClient(config);

  try {
    console.log('Application started');

    // Initialize API clients
    const storage = createStorageClient({ baseUrl: config.api?.baseUrl || 'http://localhost:8080' });
    const transaction = createTransactionClient({ baseUrl: config.api?.baseUrl || 'http://localhost:8080' });
    const monitoring = createMonitoringClient({ baseUrl: config.api?.baseUrl || 'http://localhost:8080' });

    // Check system health
    const health = await monitoring.getLivenessProbe();
    if (health.status !== 'healthy') {
      throw new Error('System is not healthy');
    }

    // Start a transaction
    const txn = await transaction.beginTransaction({
      isolation_level: IsolationLevel.READ_COMMITTED,
    });
    console.log(`Transaction ${txn.transaction_id} started`);

    try {
      // Perform database operations here...
      // For example: create tables, insert data, query data

      // Check storage utilization
      const storageStatus = await storage.getStorageStatus();
      console.log(`Storage: ${storageStatus.utilization_percent}% used`);

      // Commit transaction
      await transaction.commitTransaction(txn.transaction_id);
      console.log('Transaction committed');

    } catch (error) {
      // Rollback on error
      await transaction.rollbackTransaction(txn.transaction_id);
      console.error('Transaction rolled back:', error);
      throw error;
    }

    console.log('Application completed successfully');

  } finally {
    // Always cleanup
    await client.shutdown();
    console.log('Application shut down');
  }
}

// ============================================================================
// Example 7: Event Handling
// ============================================================================

async function example7_EventHandling() {
  console.log('\n=== Example 7: Event Handling ===\n');

  const config = createConfig()
    .autoStart(true)
    .autoStop(true)
    .logging({ stdout: false, stderr: true })
    .build();

  const client = createRustyDbClient(config);

  // Listen to events
  client.on('server:starting', () => {
    console.log('Event: Server is starting...');
  });

  client.on('server:started', () => {
    console.log('Event: Server started!');
  });

  client.on('server:error', (error) => {
    console.error('Event: Server error:', error);
  });

  client.on('connection:open', () => {
    console.log('Event: WebSocket connection opened');
  });

  client.on('connection:close', () => {
    console.log('Event: WebSocket connection closed');
  });

  try {
    await client.initialize();
    
    // Do work...
    await new Promise(resolve => setTimeout(resolve, 2000));

  } finally {
    await client.shutdown();
  }
}

// ============================================================================
// Run Examples
// ============================================================================

async function main() {
  console.log('RustyDB Node.js Adapter - Usage Examples');
  console.log('=========================================');

  // Uncomment the examples you want to run:

  // await example1_BasicUsage();
  // await example2_StorageApi();
  // await example3_Transactions();
  // await example4_Security();
  // await example5_Monitoring();
  // await example6_CompleteApplication();
  // await example7_EventHandling();

  console.log('\n✓ All examples completed successfully!\n');
}

// Run if executed directly
if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

export {
  example1_BasicUsage,
  example2_StorageApi,
  example3_Transactions,
  example4_Security,
  example5_Monitoring,
  example6_CompleteApplication,
  example7_EventHandling,
};
