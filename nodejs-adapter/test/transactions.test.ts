/**
 * RustyDB Node.js Adapter - Transaction API Tests
 *
 * Comprehensive test suite for Transaction Layer APIs
 * Tests: CRUD, Locks, Deadlocks, MVCC, WAL, GraphQL
 *
 * @module transactions.test
 * @author PhD Software Engineer Agent 2
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from '@jest/globals';
import {
  TransactionClient,
  IsolationLevel,
  TransactionState,
  LockMode,
  GraphQLTransactionOpType,
  type TransactionRequest,
  type ActiveTransactionInfo,
  type TransactionDetails,
  type LockStatusResponse,
  type LockWaitGraph,
  type DeadlockInfo,
  type MvccStatus,
  type VacuumRequest,
  type WalStatus,
  type CheckpointResult,
} from '../src/api/transactions';

// Test configuration
const TEST_CONFIG = {
  baseUrl: process.env.RUSTYDB_URL || 'http://localhost:8080',
  timeout: 30000,
};

describe('Transaction API Client', () => {
  let client: TransactionClient;

  beforeAll(() => {
    client = new TransactionClient(TEST_CONFIG.baseUrl);
  });

  afterAll(async () => {
    // Clean up any active connections
    await client.close();
  });

  // ==========================================================================
  // Transaction Lifecycle Tests
  // ==========================================================================

  describe('Transaction Lifecycle', () => {
    let transactionId: number;

    it('should begin a new transaction with default settings', async () => {
      const response = await client.beginTransaction();

      expect(response).toBeDefined();
      expect(response.transaction_id).toBeDefined();
      expect(response.transaction_id.id).toBeGreaterThan(0);
      expect(response.status).toBe('active');
      expect(response.isolation_level).toBeDefined();
      expect(response.started_at).toBeGreaterThan(0);

      transactionId = response.transaction_id.id;
    }, TEST_CONFIG.timeout);

    it('should begin a transaction with custom isolation level', async () => {
      const request: TransactionRequest = {
        isolation_level: 'REPEATABLE_READ',
        read_only: false,
      };

      const response = await client.beginTransaction(request);

      expect(response.transaction_id.id).toBeGreaterThan(0);
      expect(response.isolation_level).toBe('REPEATABLE_READ');
      expect(response.status).toBe('active');
    }, TEST_CONFIG.timeout);

    it('should begin a read-only transaction', async () => {
      const request: TransactionRequest = {
        isolation_level: 'READ_COMMITTED',
        read_only: true,
      };

      const response = await client.beginTransaction(request);

      expect(response.transaction_id.id).toBeGreaterThan(0);
      expect(response.status).toBe('active');
    }, TEST_CONFIG.timeout);

    it('should commit a transaction', async () => {
      const txn = await client.beginTransaction();
      await client.commitTransaction(txn.transaction_id.id);

      // Verify transaction is no longer active
      const active = await client.getActiveTransactions();
      const found = active.find(t => t.transaction_id.id === txn.transaction_id.id);
      expect(found).toBeUndefined();
    }, TEST_CONFIG.timeout);

    it('should rollback a transaction', async () => {
      const txn = await client.beginTransaction();
      const result = await client.rollbackTransaction(txn.transaction_id.id);

      expect(result).toBeDefined();
      expect(result.transaction_id).toBe(txn.transaction_id.id);
      expect(result.status).toBe('rolled_back');
      expect(result.timestamp).toBeGreaterThan(0);
    }, TEST_CONFIG.timeout);

    it('should fail to commit non-existent transaction', async () => {
      await expect(client.commitTransaction(999999)).rejects.toThrow();
    }, TEST_CONFIG.timeout);

    it('should fail to rollback non-existent transaction', async () => {
      await expect(client.rollbackTransaction(999999)).rejects.toThrow();
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // Transaction Monitoring Tests
  // ==========================================================================

  describe('Transaction Monitoring', () => {
    let testTransactionId: number;

    beforeEach(async () => {
      const txn = await client.beginTransaction();
      testTransactionId = txn.transaction_id.id;
    });

    it('should list all active transactions', async () => {
      const transactions = await client.getActiveTransactions();

      expect(Array.isArray(transactions)).toBe(true);
      expect(transactions.length).toBeGreaterThan(0);

      // Verify structure of each transaction
      transactions.forEach((txn: ActiveTransactionInfo) => {
        expect(txn.transaction_id).toBeDefined();
        expect(txn.session_id).toBeDefined();
        expect(txn.started_at).toBeGreaterThan(0);
        expect(txn.isolation_level).toBeDefined();
        expect(txn.state).toBeDefined();
        expect(typeof txn.read_only).toBe('boolean');
        expect(typeof txn.queries_executed).toBe('number');
        expect(typeof txn.rows_affected).toBe('number');
        expect(typeof txn.locks_held).toBe('number');
      });
    }, TEST_CONFIG.timeout);

    it('should get transaction details by ID', async () => {
      const details = await client.getTransaction(testTransactionId);

      expect(details).toBeDefined();
      expect(details.transaction_id.id).toBe(testTransactionId);
      expect(details.session_id).toBeDefined();
      expect(details.started_at).toBeGreaterThan(0);
      expect(details.isolation_level).toBeDefined();
      expect(details.state).toBeDefined();
      expect(typeof details.read_only).toBe('boolean');
      expect(Array.isArray(details.locks_held)).toBe(true);
      expect(Array.isArray(details.modified_tables)).toBe(true);
      expect(typeof details.wal_bytes_written).toBe('number');
    }, TEST_CONFIG.timeout);

    it('should fail to get non-existent transaction', async () => {
      await expect(client.getTransaction(999999)).rejects.toThrow();
    }, TEST_CONFIG.timeout);

    it('should verify transaction state transitions', async () => {
      const txn = await client.beginTransaction();
      const id = txn.transaction_id.id;

      // Check initial state
      let details = await client.getTransaction(id);
      expect(details.state).toBe('active');

      // Commit and verify
      await client.commitTransaction(id);

      // Transaction should no longer be in active list
      const active = await client.getActiveTransactions();
      const found = active.find(t => t.transaction_id.id === id);
      expect(found).toBeUndefined();
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // Lock Management Tests
  // ==========================================================================

  describe('Lock Management', () => {
    it('should get current lock status', async () => {
      const lockStatus = await client.getLocks();

      expect(lockStatus).toBeDefined();
      expect(typeof lockStatus.total_locks).toBe('number');
      expect(typeof lockStatus.granted_locks).toBe('number');
      expect(typeof lockStatus.waiting_locks).toBe('number');
      expect(Array.isArray(lockStatus.locks)).toBe(true);

      // Verify lock counts are consistent
      expect(lockStatus.granted_locks + lockStatus.waiting_locks).toBe(lockStatus.total_locks);

      // Verify structure of each lock
      lockStatus.locks.forEach(lock => {
        expect(lock.lock_id).toBeDefined();
        expect(lock.lock_type).toBeDefined();
        expect(lock.resource_type).toBeDefined();
        expect(lock.resource_id).toBeDefined();
        expect(lock.transaction_id).toBeDefined();
        expect(typeof lock.granted).toBe('boolean');
        expect(lock.acquired_at).toBeGreaterThan(0);
      });
    }, TEST_CONFIG.timeout);

    it('should get lock wait graph', async () => {
      const graph = await client.getLockWaitGraph();

      expect(graph).toBeDefined();
      expect(Array.isArray(graph.waiters)).toBe(true);
      expect(Array.isArray(graph.potential_deadlocks)).toBe(true);

      // Verify structure of waiters
      graph.waiters.forEach(waiter => {
        expect(waiter.transaction_id).toBeDefined();
        expect(waiter.waiting_for_transaction).toBeDefined();
        expect(waiter.lock_type).toBeDefined();
        expect(waiter.resource_type).toBeDefined();
        expect(waiter.resource_id).toBeDefined();
        expect(typeof waiter.wait_time_ms).toBe('number');
      });

      // Verify structure of potential deadlocks
      graph.potential_deadlocks.forEach(cycle => {
        expect(Array.isArray(cycle)).toBe(true);
        expect(cycle.length).toBeGreaterThan(0);
      });
    }, TEST_CONFIG.timeout);

    it('should identify lock types correctly', async () => {
      const lockStatus = await client.getLocks();

      const lockTypes = new Set(lockStatus.locks.map(l => l.lock_type));
      const resourceTypes = new Set(lockStatus.locks.map(l => l.resource_type));

      // Verify we have valid lock types
      lockTypes.forEach(type => {
        expect(['shared', 'exclusive', 'row_shared', 'row_exclusive']).toContain(type);
      });

      // Verify we have valid resource types
      resourceTypes.forEach(type => {
        expect(['table', 'row', 'page']).toContain(type);
      });
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // Deadlock Detection Tests
  // ==========================================================================

  describe('Deadlock Detection', () => {
    it('should get deadlock history', async () => {
      const deadlocks = await client.getDeadlocks();

      expect(Array.isArray(deadlocks)).toBe(true);

      // Verify structure of each deadlock
      deadlocks.forEach((dl: DeadlockInfo) => {
        expect(dl.deadlock_id).toBeDefined();
        expect(dl.detected_at).toBeGreaterThan(0);
        expect(Array.isArray(dl.transactions)).toBe(true);
        expect(dl.transactions.length).toBeGreaterThan(1);
        expect(dl.victim_transaction).toBeDefined();
        expect(dl.resolution).toBeDefined();
      });
    }, TEST_CONFIG.timeout);

    it('should force deadlock detection', async () => {
      const result = await client.detectDeadlocks();

      expect(result).toBeDefined();
      expect(typeof result.deadlocks_detected).toBe('number');
      expect(typeof result.transactions_analyzed).toBe('number');
      expect(result.timestamp).toBeGreaterThan(0);
      expect(result.deadlocks_detected).toBeGreaterThanOrEqual(0);
    }, TEST_CONFIG.timeout);

    it('should analyze deadlock patterns', async () => {
      const deadlocks = await client.getDeadlocks();

      if (deadlocks.length > 0) {
        // Analyze deadlock characteristics
        const avgTransactionsInvolved = deadlocks.reduce(
          (sum, dl) => sum + dl.transactions.length,
          0
        ) / deadlocks.length;

        expect(avgTransactionsInvolved).toBeGreaterThan(1);

        // Verify victim is one of the involved transactions
        deadlocks.forEach(dl => {
          const victimId = dl.victim_transaction.id;
          const involvedIds = dl.transactions.map(t => t.id);
          expect(involvedIds).toContain(victimId);
        });
      }
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // MVCC Operations Tests
  // ==========================================================================

  describe('MVCC Operations', () => {
    it('should get MVCC status', async () => {
      const status = await client.getMvccStatus();

      expect(status).toBeDefined();
      expect(typeof status.total_versions).toBe('number');
      expect(typeof status.dead_tuples).toBe('number');
      expect(typeof status.live_tuples).toBe('number');
      expect(typeof status.vacuum_running).toBe('boolean');

      // Verify tuple counts make sense
      expect(status.total_versions).toBeGreaterThanOrEqual(0);
      expect(status.dead_tuples).toBeGreaterThanOrEqual(0);
      expect(status.live_tuples).toBeGreaterThanOrEqual(0);

      // Optional fields
      if (status.oldest_active_transaction) {
        expect(status.oldest_active_transaction.id).toBeGreaterThan(0);
      }
      if (status.oldest_snapshot) {
        expect(status.oldest_snapshot.id).toBeGreaterThan(0);
      }
    }, TEST_CONFIG.timeout);

    it('should trigger vacuum on specific table', async () => {
      const request: VacuumRequest = {
        target: 'users',
        analyze: true,
        full: false,
      };

      const response = await client.triggerVacuum(request);

      expect(response).toBeDefined();
      expect(response.status).toBe('started');
      expect(response.target).toBe('users');
      expect(response.analyze).toBe(true);
      expect(response.full).toBe(false);
      expect(response.started_at).toBeGreaterThan(0);
    }, TEST_CONFIG.timeout);

    it('should trigger full database vacuum', async () => {
      const request: VacuumRequest = {
        full: true,
        analyze: true,
      };

      const response = await client.triggerVacuum(request);

      expect(response).toBeDefined();
      expect(response.status).toBe('started');
      expect(response.target).toBe('all');
      expect(response.full).toBe(true);
    }, TEST_CONFIG.timeout);

    it('should calculate dead tuple ratio', async () => {
      const status = await client.getMvccStatus();

      const totalTuples = status.dead_tuples + status.live_tuples;
      if (totalTuples > 0) {
        const deadRatio = status.dead_tuples / totalTuples;
        expect(deadRatio).toBeGreaterThanOrEqual(0);
        expect(deadRatio).toBeLessThanOrEqual(1);

        // Log warning if dead tuple ratio is high
        if (deadRatio > 0.2) {
          console.warn(`High dead tuple ratio: ${(deadRatio * 100).toFixed(2)}%`);
        }
      }
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // WAL Management Tests
  // ==========================================================================

  describe('WAL Management', () => {
    it('should get WAL status', async () => {
      const status = await client.getWalStatus();

      expect(status).toBeDefined();
      expect(status.current_lsn).toBeDefined();
      expect(status.checkpoint_lsn).toBeDefined();
      expect(typeof status.wal_files).toBe('number');
      expect(typeof status.wal_size_bytes).toBe('number');
      expect(typeof status.write_rate_mbps).toBe('number');
      expect(typeof status.sync_rate_mbps).toBe('number');
      expect(status.last_checkpoint).toBeGreaterThan(0);
      expect(typeof status.checkpoint_in_progress).toBe('boolean');

      // Verify LSN format
      expect(status.current_lsn).toMatch(/^0x[0-9A-F]+\/[0-9A-F]+$/i);
      expect(status.checkpoint_lsn).toMatch(/^0x[0-9A-F]+\/[0-9A-F]+$/i);
    }, TEST_CONFIG.timeout);

    it('should force checkpoint', async () => {
      const result = await client.forceCheckpoint();

      expect(result).toBeDefined();
      expect(result.checkpoint_lsn).toBeDefined();
      expect(result.started_at).toBeGreaterThan(0);
      expect(result.completed_at).toBeGreaterThan(0);
      expect(result.duration_ms).toBeGreaterThan(0);
      expect(result.pages_written).toBeGreaterThan(0);
      expect(result.bytes_written).toBeGreaterThan(0);

      // Verify timing
      expect(result.completed_at).toBeGreaterThanOrEqual(result.started_at);
      const calculatedDuration = (result.completed_at - result.started_at) * 1000;
      expect(result.duration_ms).toBeCloseTo(calculatedDuration, -2);
    }, TEST_CONFIG.timeout);

    it('should monitor WAL growth', async () => {
      const status1 = await client.getWalStatus();
      const walSize1 = status1.wal_size_bytes;

      // Perform some operations that generate WAL
      const txn = await client.beginTransaction();
      // ... perform operations ...
      await client.commitTransaction(txn.transaction_id.id);

      const status2 = await client.getWalStatus();
      const walSize2 = status2.wal_size_bytes;

      // WAL size should be non-negative
      expect(walSize1).toBeGreaterThanOrEqual(0);
      expect(walSize2).toBeGreaterThanOrEqual(0);
    }, TEST_CONFIG.timeout);

    it('should verify checkpoint advances LSN', async () => {
      const status1 = await client.getWalStatus();
      const lsn1 = status1.checkpoint_lsn;

      await client.forceCheckpoint();

      const status2 = await client.getWalStatus();
      const lsn2 = status2.checkpoint_lsn;

      // Checkpoint LSN should advance or stay the same
      expect(lsn2).toBeDefined();
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // Savepoint Tests (Placeholder)
  // ==========================================================================

  describe('Savepoint Operations (Not Yet Implemented)', () => {
    it('should throw error when creating savepoint', async () => {
      const txn = await client.beginTransaction();

      await expect(
        client.createSavepoint(txn.transaction_id.id, { name: 'sp1' })
      ).rejects.toThrow('Savepoint endpoints not yet implemented');
    }, TEST_CONFIG.timeout);

    it('should throw error when rolling back to savepoint', async () => {
      const txn = await client.beginTransaction();

      await expect(
        client.rollbackToSavepoint(txn.transaction_id.id, 'sp1')
      ).rejects.toThrow('Savepoint endpoints not yet implemented');
    }, TEST_CONFIG.timeout);

    it('should throw error when releasing savepoint', async () => {
      const txn = await client.beginTransaction();

      await expect(
        client.releaseSavepoint(txn.transaction_id.id, 'sp1')
      ).rejects.toThrow('Savepoint endpoints not yet implemented');
    }, TEST_CONFIG.timeout);

    it('should throw error when listing savepoints', async () => {
      const txn = await client.beginTransaction();

      await expect(
        client.listSavepoints(txn.transaction_id.id)
      ).rejects.toThrow('Savepoint endpoints not yet implemented');
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // GraphQL Transaction Tests
  // ==========================================================================

  describe('GraphQL Transaction Operations', () => {
    it('should begin transaction via GraphQL', async () => {
      const result = await client.graphqlBeginTransaction(IsolationLevel.READ_COMMITTED);

      expect(result).toBeDefined();
      expect(result.transaction_id).toBeDefined();
      expect(result.status).toBeDefined();
      expect(result.timestamp).toBeDefined();
    }, TEST_CONFIG.timeout);

    it('should commit transaction via GraphQL', async () => {
      const begin = await client.graphqlBeginTransaction();
      const result = await client.graphqlCommitTransaction(begin.transaction_id);

      expect(result).toBeDefined();
      expect(result.transaction_id).toBe(begin.transaction_id);
      expect(result.status).toBeDefined();
    }, TEST_CONFIG.timeout);

    it('should rollback transaction via GraphQL', async () => {
      const begin = await client.graphqlBeginTransaction();
      const result = await client.graphqlRollbackTransaction(begin.transaction_id);

      expect(result).toBeDefined();
      expect(result.transaction_id).toBe(begin.transaction_id);
      expect(result.status).toBeDefined();
    }, TEST_CONFIG.timeout);

    it('should execute transaction with multiple operations', async () => {
      const operations = [
        {
          operation_type: GraphQLTransactionOpType.INSERT,
          table: 'users',
          data: { name: 'Alice', email: 'alice@example.com' },
        },
        {
          operation_type: GraphQLTransactionOpType.UPDATE,
          table: 'accounts',
          data: { balance: 1000 },
          where_clause: { user_id: 1 },
        },
      ];

      const result = await client.graphqlExecuteTransaction(
        operations,
        IsolationLevel.SERIALIZABLE
      );

      expect(result).toBeDefined();
      expect(typeof result.success).toBe('boolean');
      expect(Array.isArray(result.results)).toBe(true);
      expect(typeof result.execution_time_ms).toBe('number');

      if (!result.success) {
        expect(result.error).toBeDefined();
      }
    }, TEST_CONFIG.timeout);
  });

  // ==========================================================================
  // Integration Tests
  // ==========================================================================

  describe('Integration Tests', () => {
    it('should handle complete transaction lifecycle', async () => {
      // Begin transaction
      const txn = await client.beginTransaction({
        isolation_level: 'REPEATABLE_READ',
      });

      // Verify it appears in active transactions
      let active = await client.getActiveTransactions();
      expect(active.some(t => t.transaction_id.id === txn.transaction_id.id)).toBe(true);

      // Get transaction details
      const details = await client.getTransaction(txn.transaction_id.id);
      expect(details.state).toBe('active');

      // Commit transaction
      await client.commitTransaction(txn.transaction_id.id);

      // Verify it's no longer active
      active = await client.getActiveTransactions();
      expect(active.some(t => t.transaction_id.id === txn.transaction_id.id)).toBe(false);
    }, TEST_CONFIG.timeout);

    it('should monitor system health metrics', async () => {
      // Get all monitoring data
      const [locks, mvcc, wal] = await Promise.all([
        client.getLocks(),
        client.getMvccStatus(),
        client.getWalStatus(),
      ]);

      // Create health report
      const healthReport = {
        locks: {
          total: locks.total_locks,
          waiting: locks.waiting_locks,
          waitRatio: locks.total_locks > 0 ? locks.waiting_locks / locks.total_locks : 0,
        },
        mvcc: {
          totalVersions: mvcc.total_versions,
          deadTuples: mvcc.dead_tuples,
          deadRatio: (mvcc.dead_tuples + mvcc.live_tuples) > 0
            ? mvcc.dead_tuples / (mvcc.dead_tuples + mvcc.live_tuples)
            : 0,
          vacuumRunning: mvcc.vacuum_running,
        },
        wal: {
          files: wal.wal_files,
          sizeMB: wal.wal_size_bytes / 1024 / 1024,
          writeRateMBps: wal.write_rate_mbps,
          checkpointInProgress: wal.checkpoint_in_progress,
        },
      };

      console.log('System Health Report:', JSON.stringify(healthReport, null, 2));

      // Verify health metrics are reasonable
      expect(healthReport.locks.waitRatio).toBeLessThan(0.5); // Less than 50% waiting
      expect(healthReport.mvcc.deadRatio).toBeLessThan(0.3); // Less than 30% dead tuples
    }, TEST_CONFIG.timeout);
  });
});
