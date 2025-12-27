import { get, post, buildQueryParams } from './api';
import type {
  UUID,
  Timestamp,
  Duration,
  PaginationParams,
  PaginatedResponse,
} from '../types';

// ============================================================================
// Transaction Service Types
// ============================================================================

export interface ActiveTransaction {
  id: UUID;
  transactionId: number;
  sessionId: UUID;
  userId?: string;
  database: string;
  state: TransactionState;
  isolationLevel: IsolationLevel;
  startTime: Timestamp;
  duration: Duration;
  currentQuery?: string;
  locksHeld: number;
  locksWaiting: number;
  rowsRead: number;
  rowsWritten: number;
  bytesWritten: number;
  isDirty: boolean;
  isReadOnly: boolean;
}

export type TransactionState =
  | 'active'
  | 'idle_in_transaction'
  | 'idle_in_transaction_aborted'
  | 'preparing'
  | 'prepared'
  | 'committing'
  | 'aborting';

export type IsolationLevel =
  | 'read_uncommitted'
  | 'read_committed'
  | 'repeatable_read'
  | 'serializable'
  | 'snapshot';

export interface TransactionDetails {
  id: UUID;
  transactionId: number;
  sessionId: UUID;
  userId?: string;
  username?: string;
  database: string;
  state: TransactionState;
  isolationLevel: IsolationLevel;
  startTime: Timestamp;
  duration: Duration;
  currentQuery?: string;
  queryHistory: QueryHistoryItem[];
  locks: LockInfo[];
  statistics: TransactionStatistics;
  snapshot?: SnapshotInfo;
}

export interface QueryHistoryItem {
  sql: string;
  startTime: Timestamp;
  duration?: Duration;
  status: 'running' | 'completed' | 'failed';
  rowsAffected?: number;
}

export interface TransactionStatistics {
  rowsRead: number;
  rowsInserted: number;
  rowsUpdated: number;
  rowsDeleted: number;
  bytesRead: number;
  bytesWritten: number;
  locksAcquired: number;
  locksReleased: number;
  deadlocksDetected: number;
  walBytesWritten: number;
}

export interface SnapshotInfo {
  snapshotId: number;
  timestamp: Timestamp;
  xmin: number;
  xmax: number;
  activeTransactions: number[];
}

export interface LockInfo {
  id: UUID;
  lockType: LockType;
  lockMode: LockMode;
  resourceType: ResourceType;
  resourceId: string;
  database?: string;
  table?: string;
  page?: number;
  tuple?: number;
  granted: boolean;
  waitingSince?: Timestamp;
  blockedBy?: UUID[];
}

export type LockType =
  | 'shared'
  | 'exclusive'
  | 'update'
  | 'intent_shared'
  | 'intent_exclusive'
  | 'shared_intent_exclusive';

export type LockMode =
  | 'access_share'
  | 'row_share'
  | 'row_exclusive'
  | 'share_update_exclusive'
  | 'share'
  | 'share_row_exclusive'
  | 'exclusive'
  | 'access_exclusive';

export type ResourceType =
  | 'database'
  | 'table'
  | 'page'
  | 'tuple'
  | 'transaction'
  | 'relation'
  | 'extent'
  | 'advisory';

export interface CurrentLock {
  lockId: UUID;
  transactionId: UUID;
  sessionId: UUID;
  userId?: string;
  lockType: LockType;
  lockMode: LockMode;
  resourceType: ResourceType;
  resourceName: string;
  database?: string;
  table?: string;
  granted: boolean;
  grantedAt?: Timestamp;
  waitingSince?: Timestamp;
  blockedBy?: BlockingInfo[];
}

export interface BlockingInfo {
  transactionId: UUID;
  sessionId: UUID;
  userId?: string;
  lockType: LockType;
  lockMode: LockMode;
  duration: Duration;
}

export interface DeadlockEvent {
  id: UUID;
  timestamp: Timestamp;
  detectedBy: string;
  involvedTransactions: UUID[];
  victimTransaction: UUID;
  lockGraph: LockGraphNode[];
  resolution: string;
  details: string;
}

export interface LockGraphNode {
  transactionId: UUID;
  waitsFor: UUID[];
  heldLocks: LockInfo[];
  waitingLocks: LockInfo[];
}

export interface MvccStatus {
  oldestActiveTransaction: number;
  newestCompletedTransaction: number;
  activeTransactionCount: number;
  totalSnapshots: number;
  oldestSnapshot?: Timestamp;
  totalVersions: number;
  deadTuples: number;
  liveTuples: number;
  deadTuplePercent: number;
  lastVacuum?: Timestamp;
  lastAutoVacuum?: Timestamp;
  vacuumThreshold: number;
  needsVacuum: boolean;
}

export interface VacuumRequest {
  tables?: string[];
  full?: boolean;
  analyze?: boolean;
  freeze?: boolean;
  verbose?: boolean;
}

export interface VacuumProgress {
  id: UUID;
  status: 'running' | 'completed' | 'failed';
  startTime: Timestamp;
  endTime?: Timestamp;
  tablesProcessed: number;
  totalTables: number;
  currentTable?: string;
  deadTuplesRemoved: number;
  pagesVacuumed: number;
  errors?: string[];
}

export interface TransactionFilters extends Partial<PaginationParams> {
  state?: TransactionState;
  database?: string;
  userId?: string;
  minDuration?: number;
  hasLocks?: boolean;
}

export interface LockFilters extends Partial<PaginationParams> {
  granted?: boolean;
  lockType?: LockType;
  resourceType?: ResourceType;
  database?: string;
  table?: string;
}

// ============================================================================
// Transaction Service
// ============================================================================

export const transactionService = {
  // ============================================================================
  // Active Transactions
  // ============================================================================

  /**
   * List all active transactions with optional filters
   */
  async getActiveTransactions(
    filters?: TransactionFilters
  ): Promise<PaginatedResponse<ActiveTransaction>> {
    const queryParams = filters ? buildQueryParams(filters) : '';
    const response = await get<PaginatedResponse<ActiveTransaction>>(
      `/transactions/active${queryParams}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch active transactions');
    }

    return response.data;
  },

  /**
   * Get detailed information about a specific transaction
   */
  async getTransaction(id: UUID): Promise<TransactionDetails> {
    const response = await get<TransactionDetails>(`/transactions/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch transaction details');
    }

    return response.data;
  },

  /**
   * Force rollback of a transaction
   */
  async rollbackTransaction(id: UUID): Promise<{ success: boolean; message: string }> {
    const response = await post<{ success: boolean; message: string }>(
      `/transactions/${id}/rollback`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to rollback transaction');
    }

    return response.data;
  },

  // ============================================================================
  // Locks
  // ============================================================================

  /**
   * Get all current locks in the system
   */
  async getCurrentLocks(filters?: LockFilters): Promise<PaginatedResponse<CurrentLock>> {
    const queryParams = filters ? buildQueryParams(filters) : '';
    const response = await get<PaginatedResponse<CurrentLock>>(
      `/transactions/locks${queryParams}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch current locks');
    }

    return response.data;
  },

  /**
   * Get lock statistics and summary
   */
  async getLockStatistics(): Promise<{
    totalLocks: number;
    grantedLocks: number;
    waitingLocks: number;
    locksByType: Record<LockType, number>;
    locksByMode: Record<LockMode, number>;
    locksByResource: Record<ResourceType, number>;
    averageWaitTime: Duration;
    longestWait?: Duration;
  }> {
    const response = await get<{
      totalLocks: number;
      grantedLocks: number;
      waitingLocks: number;
      locksByType: Record<LockType, number>;
      locksByMode: Record<LockMode, number>;
      locksByResource: Record<ResourceType, number>;
      averageWaitTime: Duration;
      longestWait?: Duration;
    }>('/transactions/locks/statistics');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch lock statistics');
    }

    return response.data;
  },

  // ============================================================================
  // Deadlocks
  // ============================================================================

  /**
   * Get deadlock detection history
   */
  async getDeadlocks(limit = 50): Promise<DeadlockEvent[]> {
    const response = await get<DeadlockEvent[]>(`/transactions/deadlocks?limit=${limit}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch deadlock history');
    }

    return response.data;
  },

  /**
   * Get a single deadlock event by ID
   */
  async getDeadlock(id: UUID): Promise<DeadlockEvent> {
    const response = await get<DeadlockEvent>(`/transactions/deadlocks/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch deadlock event');
    }

    return response.data;
  },

  /**
   * Get deadlock statistics
   */
  async getDeadlockStatistics(): Promise<{
    totalDeadlocks: number;
    deadlocksToday: number;
    deadlocksThisWeek: number;
    deadlocksThisMonth: number;
    mostFrequentTables: Array<{ table: string; count: number }>;
    averageResolutionTime: Duration;
  }> {
    const response = await get<{
      totalDeadlocks: number;
      deadlocksToday: number;
      deadlocksThisWeek: number;
      deadlocksThisMonth: number;
      mostFrequentTables: Array<{ table: string; count: number }>;
      averageResolutionTime: Duration;
    }>('/transactions/deadlocks/statistics');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch deadlock statistics');
    }

    return response.data;
  },

  // ============================================================================
  // MVCC (Multi-Version Concurrency Control)
  // ============================================================================

  /**
   * Get MVCC system status
   */
  async getMvccStatus(): Promise<MvccStatus> {
    const response = await get<MvccStatus>('/transactions/mvcc/status');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch MVCC status');
    }

    return response.data;
  },

  /**
   * Trigger manual vacuum operation
   */
  async triggerVacuum(request?: VacuumRequest): Promise<VacuumProgress> {
    const response = await post<VacuumProgress>('/transactions/mvcc/vacuum', request || {});

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to trigger vacuum');
    }

    return response.data;
  },

  /**
   * Get vacuum progress for a running vacuum operation
   */
  async getVacuumProgress(id: UUID): Promise<VacuumProgress> {
    const response = await get<VacuumProgress>(`/transactions/mvcc/vacuum/${id}/progress`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch vacuum progress');
    }

    return response.data;
  },

  /**
   * Get vacuum history
   */
  async getVacuumHistory(limit = 50): Promise<VacuumProgress[]> {
    const response = await get<VacuumProgress[]>(
      `/transactions/mvcc/vacuum/history?limit=${limit}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch vacuum history');
    }

    return response.data;
  },
};
