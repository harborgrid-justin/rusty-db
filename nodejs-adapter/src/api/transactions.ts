/**
 * RustyDB Node.js Adapter - Transaction API
 *
 * Comprehensive TypeScript client for RustyDB Transaction Layer APIs
 * Coverage includes: CRUD, Savepoints, Locks, Deadlocks, MVCC, WAL
 *
 * @module transactions
 * @author PhD Software Engineer Agent 2
 */

import { BaseClient } from './base-client';

// ============================================================================
// TypeScript Interfaces - Core Transaction Types
// ============================================================================

/**
 * Transaction ID type (newtype pattern)
 */
export interface TransactionId {
  id: number;
}

/**
 * Session ID type
 */
export interface SessionId {
  id: number;
}

/**
 * Transaction isolation levels
 * Matches Rust enum: IsolationLevel
 */
export enum IsolationLevel {
  READ_UNCOMMITTED = 'READ_UNCOMMITTED',
  READ_COMMITTED = 'READ_COMMITTED',
  REPEATABLE_READ = 'REPEATABLE_READ',
  SERIALIZABLE = 'SERIALIZABLE',
  SNAPSHOT_ISOLATION = 'SNAPSHOT_ISOLATION',
}

/**
 * Transaction states
 */
export enum TransactionState {
  ACTIVE = 'active',
  PREPARING = 'preparing',
  PREPARED = 'prepared',
  COMMITTED = 'committed',
  ABORTED = 'aborted',
}

/**
 * Lock modes
 */
export enum LockMode {
  SHARED = 'shared',
  EXCLUSIVE = 'exclusive',
  ROW_SHARED = 'row_shared',
  ROW_EXCLUSIVE = 'row_exclusive',
}

/**
 * Lock resource types
 */
export enum LockResourceType {
  TABLE = 'table',
  ROW = 'row',
  PAGE = 'page',
}

// ============================================================================
// REST API Request/Response Types
// ============================================================================

/**
 * Transaction request (begin transaction)
 */
export interface TransactionRequest {
  isolation_level?: string;
  read_only?: boolean;
}

/**
 * Transaction response
 */
export interface TransactionResponse {
  transaction_id: TransactionId;
  isolation_level: string;
  started_at: number;
  status: string;
}

/**
 * Active transaction information
 * Matches Rust struct: ActiveTransactionInfo
 */
export interface ActiveTransactionInfo {
  transaction_id: TransactionId;
  session_id: SessionId;
  started_at: number;
  isolation_level: string;
  state: string;
  read_only: boolean;
  queries_executed: number;
  rows_affected: number;
  locks_held: number;
}

/**
 * Lock information
 * Matches Rust struct: LockInfo
 */
export interface LockInfo {
  lock_id: string;
  lock_type: string;
  resource_type: string;
  resource_id: string;
  transaction_id: TransactionId;
  granted: boolean;
  acquired_at: number;
}

/**
 * Transaction details (full info)
 * Matches Rust struct: TransactionDetails
 */
export interface TransactionDetails {
  transaction_id: TransactionId;
  session_id: SessionId;
  started_at: number;
  isolation_level: string;
  state: string;
  read_only: boolean;
  queries_executed: number;
  rows_affected: number;
  locks_held: LockInfo[];
  modified_tables: string[];
  wal_bytes_written: number;
}

/**
 * Lock status response
 * Matches Rust struct: LockStatusResponse
 */
export interface LockStatusResponse {
  total_locks: number;
  granted_locks: number;
  waiting_locks: number;
  locks: LockInfo[];
}

/**
 * Lock waiter information
 * Matches Rust struct: LockWaiter
 */
export interface LockWaiter {
  transaction_id: TransactionId;
  waiting_for_transaction: TransactionId;
  lock_type: string;
  resource_type: string;
  resource_id: string;
  wait_time_ms: number;
}

/**
 * Lock wait graph
 * Matches Rust struct: LockWaitGraph
 */
export interface LockWaitGraph {
  waiters: LockWaiter[];
  potential_deadlocks: TransactionId[][];
}

/**
 * Deadlock information
 * Matches Rust struct: DeadlockInfo
 */
export interface DeadlockInfo {
  deadlock_id: string;
  detected_at: number;
  transactions: TransactionId[];
  victim_transaction: TransactionId;
  resolution: string;
}

/**
 * MVCC status
 * Matches Rust struct: MvccStatus
 */
export interface MvccStatus {
  oldest_active_transaction: TransactionId | null;
  oldest_snapshot: TransactionId | null;
  total_versions: number;
  dead_tuples: number;
  live_tuples: number;
  vacuum_running: boolean;
  last_vacuum: number | null;
}

/**
 * Vacuum request
 * Matches Rust struct: VacuumRequest
 */
export interface VacuumRequest {
  target?: string;
  analyze?: boolean;
  full?: boolean;
}

/**
 * Vacuum response
 */
export interface VacuumResponse {
  status: string;
  target: string;
  analyze: boolean;
  full: boolean;
  started_at: number;
}

/**
 * WAL (Write-Ahead Log) status
 * Matches Rust struct: WalStatus
 */
export interface WalStatus {
  current_lsn: string;
  checkpoint_lsn: string;
  wal_files: number;
  wal_size_bytes: number;
  write_rate_mbps: number;
  sync_rate_mbps: number;
  last_checkpoint: number;
  checkpoint_in_progress: boolean;
}

/**
 * Checkpoint result
 * Matches Rust struct: CheckpointResult
 */
export interface CheckpointResult {
  checkpoint_lsn: string;
  started_at: number;
  completed_at: number;
  duration_ms: number;
  pages_written: number;
  bytes_written: number;
}

/**
 * Deadlock detection result
 */
export interface DeadlockDetectionResult {
  deadlocks_detected: number;
  transactions_analyzed: number;
  timestamp: number;
}

/**
 * Rollback result
 */
export interface RollbackResult {
  transaction_id: number;
  status: string;
  timestamp: number;
}

/**
 * Savepoint (placeholder - for future implementation)
 * Based on Rust type: Savepoint
 */
export interface Savepoint {
  name: string;
  transaction_id: TransactionId;
  created_at: number;
  sequence_number: number;
}

/**
 * Savepoint request
 */
export interface SavepointRequest {
  name: string;
}

/**
 * Savepoint response
 */
export interface SavepointResponse {
  savepoint: Savepoint;
  status: string;
}

// ============================================================================
// GraphQL Types
// ============================================================================

/**
 * GraphQL Transaction Result
 */
export interface GraphQLTransactionResult {
  transaction_id: string;
  status: string;
  timestamp: string;
}

/**
 * GraphQL Transaction Operation Type
 */
export enum GraphQLTransactionOpType {
  INSERT = 'Insert',
  UPDATE = 'Update',
  DELETE = 'Delete',
}

/**
 * GraphQL Transaction Operation
 */
export interface GraphQLTransactionOperation {
  operation_type: GraphQLTransactionOpType;
  table: string;
  data?: Record<string, unknown>;
  where_clause?: unknown;
  id?: string;
}

/**
 * GraphQL Transaction Execution Result
 */
export interface GraphQLTransactionExecutionResult {
  success: boolean;
  results: string[];
  execution_time_ms: number;
  error: string | null;
}

// ============================================================================
// Transaction API Client
// ============================================================================

/**
 * Transaction API Client
 *
 * Provides complete coverage of all RustyDB transaction endpoints:
 * - Transaction lifecycle (begin, commit, rollback)
 * - Active transaction monitoring
 * - Lock management and monitoring
 * - Deadlock detection and history
 * - MVCC operations and status
 * - WAL management and checkpointing
 * - Savepoints (placeholder for future backend implementation)
 * - GraphQL transaction operations
 */
export class TransactionClient extends BaseClient {

  // ==========================================================================
  // Transaction Lifecycle - REST API
  // ==========================================================================

  /**
   * Begin a new transaction
   *
   * REST: POST /api/v1/transactions
   *
   * @param request - Transaction configuration
   * @returns Transaction response with ID and metadata
   *
   * @example
   * ```typescript
   * const txn = await client.beginTransaction({
   *   isolation_level: 'REPEATABLE_READ',
   *   read_only: false
   * });
   * console.log(`Started transaction ${txn.transaction_id.id}`);
   * ```
   */
  async beginTransaction(request: TransactionRequest = {}): Promise<TransactionResponse> {
    return this.post<TransactionResponse>('/api/v1/transactions', request);
  }

  /**
   * Commit a transaction
   *
   * REST: POST /api/v1/transactions/{id}/commit
   *
   * @param transactionId - Transaction ID to commit
   *
   * @example
   * ```typescript
   * await client.commitTransaction(123);
   * console.log('Transaction committed successfully');
   * ```
   */
  async commitTransaction(transactionId: number): Promise<void> {
    await this.post<void>(`/api/v1/transactions/${transactionId}/commit`, {});
  }

  /**
   * Rollback a transaction
   *
   * REST: POST /api/v1/transactions/{id}/rollback
   *
   * @param transactionId - Transaction ID to rollback
   * @returns Rollback result
   *
   * @example
   * ```typescript
   * const result = await client.rollbackTransaction(123);
   * console.log(`Transaction ${result.transaction_id} rolled back`);
   * ```
   */
  async rollbackTransaction(transactionId: number): Promise<RollbackResult> {
    return this.post<RollbackResult>(`/api/v1/transactions/${transactionId}/rollback`, {});
  }

  // ==========================================================================
  // Transaction Monitoring - REST API
  // ==========================================================================

  /**
   * Get all active transactions
   *
   * REST: GET /api/v1/transactions/active
   *
   * @returns List of active transactions
   *
   * @example
   * ```typescript
   * const active = await client.getActiveTransactions();
   * console.log(`${active.length} active transactions`);
   * active.forEach(txn => {
   *   console.log(`TXN ${txn.transaction_id.id}: ${txn.queries_executed} queries`);
   * });
   * ```
   */
  async getActiveTransactions(): Promise<ActiveTransactionInfo[]> {
    return this.get<ActiveTransactionInfo[]>('/api/v1/transactions/active');
  }

  /**
   * Get transaction details by ID
   *
   * REST: GET /api/v1/transactions/{id}
   *
   * @param transactionId - Transaction ID
   * @returns Full transaction details including locks
   *
   * @example
   * ```typescript
   * const details = await client.getTransaction(123);
   * console.log(`Transaction ${details.transaction_id.id}`);
   * console.log(`State: ${details.state}`);
   * console.log(`Locks held: ${details.locks_held.length}`);
   * console.log(`Modified tables: ${details.modified_tables.join(', ')}`);
   * ```
   */
  async getTransaction(transactionId: number): Promise<TransactionDetails> {
    return this.get<TransactionDetails>(`/api/v1/transactions/${transactionId}`);
  }

  // ==========================================================================
  // Lock Management - REST API
  // ==========================================================================

  /**
   * Get current lock status
   *
   * REST: GET /api/v1/transactions/locks
   *
   * @returns Lock status with all locks
   *
   * @example
   * ```typescript
   * const locks = await client.getLocks();
   * console.log(`Total locks: ${locks.total_locks}`);
   * console.log(`Granted: ${locks.granted_locks}`);
   * console.log(`Waiting: ${locks.waiting_locks}`);
   * ```
   */
  async getLocks(): Promise<LockStatusResponse> {
    return this.get<LockStatusResponse>('/api/v1/transactions/locks');
  }

  /**
   * Get lock wait graph
   *
   * REST: GET /api/v1/transactions/locks/waiters
   *
   * @returns Lock wait graph with potential deadlocks
   *
   * @example
   * ```typescript
   * const graph = await client.getLockWaitGraph();
   * console.log(`Waiters: ${graph.waiters.length}`);
   * console.log(`Potential deadlocks: ${graph.potential_deadlocks.length}`);
   * graph.waiters.forEach(waiter => {
   *   console.log(`TXN ${waiter.transaction_id.id} waiting for ${waiter.waiting_for_transaction.id}`);
   * });
   * ```
   */
  async getLockWaitGraph(): Promise<LockWaitGraph> {
    return this.get<LockWaitGraph>('/api/v1/transactions/locks/waiters');
  }

  // ==========================================================================
  // Deadlock Detection - REST API
  // ==========================================================================

  /**
   * Get deadlock history
   *
   * REST: GET /api/v1/transactions/deadlocks
   *
   * @returns List of detected deadlocks
   *
   * @example
   * ```typescript
   * const deadlocks = await client.getDeadlocks();
   * deadlocks.forEach(dl => {
   *   console.log(`Deadlock ${dl.deadlock_id} at ${new Date(dl.detected_at * 1000)}`);
   *   console.log(`Involved transactions: ${dl.transactions.map(t => t.id).join(', ')}`);
   *   console.log(`Victim: ${dl.victim_transaction.id}`);
   * });
   * ```
   */
  async getDeadlocks(): Promise<DeadlockInfo[]> {
    return this.get<DeadlockInfo[]>('/api/v1/transactions/deadlocks');
  }

  /**
   * Force deadlock detection
   *
   * REST: POST /api/v1/transactions/deadlocks/detect
   *
   * @returns Deadlock detection result
   *
   * @example
   * ```typescript
   * const result = await client.detectDeadlocks();
   * console.log(`Detected ${result.deadlocks_detected} deadlocks`);
   * console.log(`Analyzed ${result.transactions_analyzed} transactions`);
   * ```
   */
  async detectDeadlocks(): Promise<DeadlockDetectionResult> {
    return this.post<DeadlockDetectionResult>('/api/v1/transactions/deadlocks/detect', {});
  }

  // ==========================================================================
  // MVCC Operations - REST API
  // ==========================================================================

  /**
   * Get MVCC status
   *
   * REST: GET /api/v1/transactions/mvcc/status
   *
   * @returns MVCC statistics including version and tuple counts
   *
   * @example
   * ```typescript
   * const mvcc = await client.getMvccStatus();
   * console.log(`Total versions: ${mvcc.total_versions}`);
   * console.log(`Live tuples: ${mvcc.live_tuples}`);
   * console.log(`Dead tuples: ${mvcc.dead_tuples}`);
   * console.log(`Vacuum running: ${mvcc.vacuum_running}`);
   * if (mvcc.oldest_active_transaction) {
   *   console.log(`Oldest active: ${mvcc.oldest_active_transaction.id}`);
   * }
   * ```
   */
  async getMvccStatus(): Promise<MvccStatus> {
    return this.get<MvccStatus>('/api/v1/transactions/mvcc/status');
  }

  /**
   * Trigger vacuum operation
   *
   * REST: POST /api/v1/transactions/mvcc/vacuum
   *
   * @param request - Vacuum configuration
   * @returns Vacuum operation response
   *
   * @example
   * ```typescript
   * // Vacuum specific table
   * await client.triggerVacuum({ target: 'users', analyze: true });
   *
   * // Full database vacuum
   * await client.triggerVacuum({ full: true, analyze: true });
   * ```
   */
  async triggerVacuum(request: VacuumRequest = {}): Promise<VacuumResponse> {
    return this.post<VacuumResponse>('/api/v1/transactions/mvcc/vacuum', request);
  }

  // ==========================================================================
  // WAL Management - REST API
  // ==========================================================================

  /**
   * Get WAL (Write-Ahead Log) status
   *
   * REST: GET /api/v1/transactions/wal/status
   *
   * @returns WAL statistics and status
   *
   * @example
   * ```typescript
   * const wal = await client.getWalStatus();
   * console.log(`Current LSN: ${wal.current_lsn}`);
   * console.log(`Checkpoint LSN: ${wal.checkpoint_lsn}`);
   * console.log(`WAL files: ${wal.wal_files}`);
   * console.log(`WAL size: ${(wal.wal_size_bytes / 1024 / 1024).toFixed(2)} MB`);
   * console.log(`Write rate: ${wal.write_rate_mbps.toFixed(2)} MB/s`);
   * console.log(`Checkpoint in progress: ${wal.checkpoint_in_progress}`);
   * ```
   */
  async getWalStatus(): Promise<WalStatus> {
    return this.get<WalStatus>('/api/v1/transactions/wal/status');
  }

  /**
   * Force WAL checkpoint
   *
   * REST: POST /api/v1/transactions/wal/checkpoint
   *
   * @returns Checkpoint result with statistics
   *
   * @example
   * ```typescript
   * const result = await client.forceCheckpoint();
   * console.log(`Checkpoint completed in ${result.duration_ms}ms`);
   * console.log(`Pages written: ${result.pages_written}`);
   * console.log(`Bytes written: ${(result.bytes_written / 1024 / 1024).toFixed(2)} MB`);
   * console.log(`New checkpoint LSN: ${result.checkpoint_lsn}`);
   * ```
   */
  async forceCheckpoint(): Promise<CheckpointResult> {
    return this.post<CheckpointResult>('/api/v1/transactions/wal/checkpoint', {});
  }

  // ==========================================================================
  // Savepoint Operations - REST API (Placeholder)
  // ==========================================================================

  /**
   * Create a savepoint
   *
   * NOTE: Backend endpoints not yet implemented in transaction_handlers.rs
   * This is a placeholder for future implementation.
   *
   * Expected REST: POST /api/v1/transactions/{id}/savepoints
   *
   * @param transactionId - Transaction ID
   * @param request - Savepoint name
   * @returns Savepoint response
   */
  async createSavepoint(transactionId: number, request: SavepointRequest): Promise<SavepointResponse> {
    throw new Error(`Savepoint endpoints not yet implemented in backend for transaction ${transactionId} with name ${request.name}`);
    // return this.post<SavepointResponse>(
    //   `/api/v1/transactions/${transactionId}/savepoints`,
    //   request
    // );
  }

  /**
   * Rollback to a savepoint
   *
   * NOTE: Backend endpoints not yet implemented in transaction_handlers.rs
   * This is a placeholder for future implementation.
   *
   * Expected REST: POST /api/v1/transactions/{id}/savepoints/{name}/rollback
   *
   * @param transactionId - Transaction ID
   * @param savepointName - Savepoint name to rollback to
   */
  async rollbackToSavepoint(transactionId: number, savepointName: string): Promise<void> {
    throw new Error(`Savepoint rollback not yet implemented in backend for transaction ${transactionId}, savepoint ${savepointName}`);
    // await this.post<void>(
    //   `/api/v1/transactions/${transactionId}/savepoints/${savepointName}/rollback`,
    //   {}
    // );
  }

  /**
   * Release a savepoint
   *
   * NOTE: Backend endpoints not yet implemented in transaction_handlers.rs
   * This is a placeholder for future implementation.
   *
   * Expected REST: DELETE /api/v1/transactions/{id}/savepoints/{name}
   *
   * @param transactionId - Transaction ID
   * @param savepointName - Savepoint name to release
   */
  async releaseSavepoint(transactionId: number, savepointName: string): Promise<void> {
    throw new Error(`Savepoint release not yet implemented in backend for transaction ${transactionId}, savepoint ${savepointName}`);
    // await this.delete<void>(
    //   `/api/v1/transactions/${transactionId}/savepoints/${savepointName}`
    // );
  }

  /**
   * List savepoints for a transaction
   *
   * NOTE: Backend endpoints not yet implemented in transaction_handlers.rs
   * This is a placeholder for future implementation.
   *
   * Expected REST: GET /api/v1/transactions/{id}/savepoints
   *
   * @param transactionId - Transaction ID
   * @returns List of savepoints
   */
  async listSavepoints(transactionId: number): Promise<Savepoint[]> {
    throw new Error(`Savepoint listing not yet implemented in backend for transaction ${transactionId}`);
    // return this.get<Savepoint[]>(`/api/v1/transactions/${transactionId}/savepoints`);
  }

  // ==========================================================================
  // GraphQL Transaction Operations
  // ==========================================================================

  /**
   * Begin transaction via GraphQL
   *
   * GraphQL Mutation: beginTransaction
   *
   * @param isolationLevel - Optional isolation level
   * @returns GraphQL transaction result
   *
   * @example
   * ```typescript
   * const result = await client.graphql.beginTransaction('REPEATABLE_READ');
   * console.log(`Transaction ${result.transaction_id} started`);
   * ```
   */
  async graphqlBeginTransaction(isolationLevel?: IsolationLevel): Promise<GraphQLTransactionResult> {
    const mutation = `
      mutation BeginTransaction($isolationLevel: IsolationLevel) {
        beginTransaction(isolationLevel: $isolationLevel) {
          transaction_id
          status
          timestamp
        }
      }
    `;

    const variables = isolationLevel ? { isolationLevel } : {};
    const response = await this.graphql<{ beginTransaction: GraphQLTransactionResult }>(
      mutation,
      variables
    );

    return response.beginTransaction;
  }

  /**
   * Commit transaction via GraphQL
   *
   * GraphQL Mutation: commitTransaction
   *
   * @param transactionId - Transaction ID
   * @returns GraphQL transaction result
   *
   * @example
   * ```typescript
   * const result = await client.graphql.commitTransaction('123');
   * console.log(`Transaction committed: ${result.status}`);
   * ```
   */
  async graphqlCommitTransaction(transactionId: string): Promise<GraphQLTransactionResult> {
    const mutation = `
      mutation CommitTransaction($transactionId: String!) {
        commitTransaction(transaction_id: $transactionId) {
          transaction_id
          status
          timestamp
        }
      }
    `;

    const response = await this.graphql<{ commitTransaction: GraphQLTransactionResult }>(
      mutation,
      { transactionId }
    );

    return response.commitTransaction;
  }

  /**
   * Rollback transaction via GraphQL
   *
   * GraphQL Mutation: rollbackTransaction
   *
   * @param transactionId - Transaction ID
   * @returns GraphQL transaction result
   *
   * @example
   * ```typescript
   * const result = await client.graphql.rollbackTransaction('123');
   * console.log(`Transaction rolled back: ${result.status}`);
   * ```
   */
  async graphqlRollbackTransaction(transactionId: string): Promise<GraphQLTransactionResult> {
    const mutation = `
      mutation RollbackTransaction($transactionId: String!) {
        rollbackTransaction(transaction_id: $transactionId) {
          transaction_id
          status
          timestamp
        }
      }
    `;

    const response = await this.graphql<{ rollbackTransaction: GraphQLTransactionResult }>(
      mutation,
      { transactionId }
    );

    return response.rollbackTransaction;
  }

  /**
   * Execute multiple operations in a transaction via GraphQL
   *
   * GraphQL Mutation: executeTransaction
   *
   * @param operations - List of operations to execute
   * @param isolationLevel - Optional isolation level
   * @returns Transaction execution result
   *
   * @example
   * ```typescript
   * const result = await client.graphql.executeTransaction([
   *   {
   *     operation_type: GraphQLTransactionOpType.INSERT,
   *     table: 'users',
   *     data: { name: 'Alice', email: 'alice@example.com' }
   *   },
   *   {
   *     operation_type: GraphQLTransactionOpType.UPDATE,
   *     table: 'accounts',
   *     data: { balance: 1000 },
   *     where_clause: { user_id: 123 }
   *   }
   * ], IsolationLevel.SERIALIZABLE);
   *
   * if (result.success) {
   *   console.log(`Transaction completed in ${result.execution_time_ms}ms`);
   * } else {
   *   console.error(`Transaction failed: ${result.error}`);
   * }
   * ```
   */
  async graphqlExecuteTransaction(
    operations: GraphQLTransactionOperation[],
    isolationLevel?: IsolationLevel
  ): Promise<GraphQLTransactionExecutionResult> {
    const mutation = `
      mutation ExecuteTransaction(
        $operations: [TransactionOperation!]!,
        $isolationLevel: IsolationLevel
      ) {
        executeTransaction(
          operations: $operations,
          isolation_level: $isolationLevel
        ) {
          success
          results
          execution_time_ms
          error
        }
      }
    `;

    const response = await this.graphql<{ executeTransaction: GraphQLTransactionExecutionResult }>(
      mutation,
      { operations, isolationLevel }
    );

    return response.executeTransaction;
  }
}

// ============================================================================
// Convenience Export
// ============================================================================

export default TransactionClient;
