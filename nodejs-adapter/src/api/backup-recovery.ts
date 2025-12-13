/**
 * RustyDB Backup & Recovery API Client
 *
 * Comprehensive TypeScript adapter for ALL Backup & Recovery endpoints including:
 * - Full and incremental backups
 * - Restore operations with PITR (Point-in-Time Recovery)
 * - Flashback operations (query, table, database, transaction)
 * - Restore points management
 * - Version queries (time-travel)
 * - Backup scheduling
 *
 * API Version: v1
 * Base Path: /api/v1
 */

import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';

// ============================================================================
// Type Definitions
// ============================================================================

/**
 * Backup Types
 */
export type BackupType = 'full' | 'incremental';
export type BackupStatus = 'in_progress' | 'completed' | 'failed' | 'cancelled';
export type RestoreStatus = 'in_progress' | 'completed' | 'failed' | 'cancelled';
export type FlashbackOperation = 'INSERT' | 'UPDATE' | 'DELETE';

/**
 * Backup Configuration
 */
export interface CreateBackupRequest {
  backup_type: BackupType;
  compression?: boolean;
  encryption?: boolean;
  destination?: string;
  retention_days?: number;
  description?: string;
}

/**
 * Full Backup Details
 */
export interface BackupDetails {
  backup_id: string;
  backup_type: BackupType;
  status: BackupStatus;
  database_name: string;
  start_time: number; // Unix timestamp (seconds)
  completion_time?: number;
  size_bytes?: number;
  compressed_size_bytes?: number;
  location: string;
  compression_enabled: boolean;
  encryption_enabled: boolean;
  retention_until?: number;
  description?: string;
  error_message?: string;
}

/**
 * Backup List Response
 */
export interface BackupList {
  backups: BackupSummary[];
  total_count: number;
}

/**
 * Backup Summary (for list views)
 */
export interface BackupSummary {
  backup_id: string;
  backup_type: BackupType;
  status: BackupStatus;
  start_time: number;
  size_bytes?: number;
  location: string;
}

/**
 * Restore Request
 */
export interface RestoreRequest {
  target_database?: string;
  point_in_time?: number; // Unix timestamp for PITR
  verify_only?: boolean;
  overwrite_existing?: boolean;
}

/**
 * Restore Response
 */
export interface RestoreResponse {
  restore_id: string;
  status: RestoreStatus;
  message: string;
  started_at: number;
}

/**
 * Backup Schedule Configuration
 */
export interface BackupSchedule {
  enabled: boolean;
  full_backup_cron: string; // Cron expression
  incremental_backup_cron: string; // Cron expression
  retention_days: number;
  compression: boolean;
  encryption: boolean;
  destination: string;
}

/**
 * Flashback Query Request (AS OF timestamp/SCN)
 */
export interface FlashbackQueryRequest {
  table: string;
  timestamp?: string; // ISO 8601 format
  scn?: number; // System Change Number
  columns?: string[];
  filter?: Record<string, any>;
  limit?: number;
}

/**
 * Flashback Query Response
 */
export interface FlashbackQueryResponse {
  rows: Array<Record<string, any>>;
  count: number;
  query_scn: number;
  query_timestamp: number;
}

/**
 * Flashback Table Request
 */
export interface FlashbackTableRequest {
  table: string;
  target_timestamp?: string; // ISO 8601
  target_scn?: number;
  restore_point?: string;
  enable_triggers?: boolean;
}

/**
 * Flashback Table Response
 */
export interface FlashbackTableResponse {
  table: string;
  status: string;
  rows_restored: number;
  restore_timestamp: number;
  duration_ms: number;
}

/**
 * Versions Query Request (row history)
 */
export interface VersionsQueryRequest {
  table: string;
  primary_key: Record<string, any>;
  start_scn?: number;
  end_scn?: number;
  start_timestamp?: string;
  end_timestamp?: string;
}

/**
 * Row Version Entry
 */
export interface RowVersion {
  scn: number;
  timestamp: number;
  operation: FlashbackOperation;
  transaction_id: string;
  data: Record<string, any>;
  changed_columns?: string[];
}

/**
 * Versions Query Response
 */
export interface VersionsQueryResponse {
  versions: RowVersion[];
  count: number;
}

/**
 * Create Restore Point Request
 */
export interface CreateRestorePointRequest {
  name: string;
  guaranteed?: boolean;
  preserve_logs?: boolean;
}

/**
 * Restore Point Response
 */
export interface RestorePointResponse {
  name: string;
  scn: number;
  timestamp: number;
  guaranteed: boolean;
}

/**
 * Restore Point Info
 */
export interface RestorePointInfo {
  name: string;
  scn: number;
  timestamp: number;
  guaranteed: boolean;
}

/**
 * Flashback Database Request
 */
export interface FlashbackDatabaseRequest {
  target_timestamp?: string;
  target_scn?: number;
  restore_point?: string;
}

/**
 * Flashback Database Response
 */
export interface FlashbackDatabaseResponse {
  status: string;
  target_scn: number;
  target_timestamp: number;
  duration_ms: number;
}

/**
 * Flashback Statistics
 */
export interface FlashbackStatsResponse {
  current_scn: number;
  oldest_scn: number;
  retention_days: number;
  total_versions: number;
  storage_bytes: number;
  queries_executed: number;
  restore_points: RestorePointInfo[];
}

/**
 * Transaction Flashback Request
 */
export interface TransactionFlashbackRequest {
  transaction_id: string;
  cascade?: boolean;
}

/**
 * Transaction Flashback Response
 */
export interface TransactionFlashbackResponse {
  transaction_id: string;
  status: string;
  operations_reversed: number;
  affected_tables: string[];
}

/**
 * Point-in-Time Recovery Configuration
 */
export interface PITRConfig {
  enabled: boolean;
  retention_days: number;
  wal_archive_location: string;
  continuous_archiving: boolean;
}

/**
 * API Error Response
 */
export interface ApiError {
  code: string;
  message: string;
  details?: any;
}

// ============================================================================
// Backup & Recovery API Client
// ============================================================================

export interface BackupRecoveryClientConfig {
  baseURL: string;
  timeout?: number;
  headers?: Record<string, string>;
}

export class BackupRecoveryClient {
  private client: AxiosInstance;

  constructor(config: BackupRecoveryClientConfig) {
    this.client = axios.create({
      baseURL: config.baseURL,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...config.headers,
      },
    });
  }

  // ==========================================================================
  // Backup Operations
  // ==========================================================================

  /**
   * Create a full backup
   * POST /api/v1/backup/full
   */
  async createFullBackup(request: CreateBackupRequest): Promise<BackupDetails> {
    const response = await this.client.post<BackupDetails>(
      '/api/v1/backup/full',
      request
    );
    return response.data;
  }

  /**
   * Create an incremental backup
   * POST /api/v1/backup/incremental
   */
  async createIncrementalBackup(request: CreateBackupRequest): Promise<BackupDetails> {
    const response = await this.client.post<BackupDetails>(
      '/api/v1/backup/incremental',
      request
    );
    return response.data;
  }

  /**
   * Create a backup (generic method)
   */
  async createBackup(
    backupType: BackupType,
    request: Omit<CreateBackupRequest, 'backup_type'>
  ): Promise<BackupDetails> {
    const fullRequest: CreateBackupRequest = {
      ...request,
      backup_type: backupType,
    };

    if (backupType === 'full') {
      return this.createFullBackup(fullRequest);
    } else {
      return this.createIncrementalBackup(fullRequest);
    }
  }

  /**
   * List all backups
   * GET /api/v1/backup/list
   */
  async listBackups(): Promise<BackupList> {
    const response = await this.client.get<BackupList>('/api/v1/backup/list');
    return response.data;
  }

  /**
   * Get backup details by ID
   * GET /api/v1/backup/{id}
   */
  async getBackup(backupId: string): Promise<BackupDetails> {
    const response = await this.client.get<BackupDetails>(
      `/api/v1/backup/${backupId}`
    );
    return response.data;
  }

  /**
   * Restore from backup
   * POST /api/v1/backup/{id}/restore
   */
  async restoreBackup(
    backupId: string,
    request: RestoreRequest
  ): Promise<RestoreResponse> {
    const response = await this.client.post<RestoreResponse>(
      `/api/v1/backup/${backupId}/restore`,
      request
    );
    return response.data;
  }

  /**
   * Restore to point-in-time (PITR)
   */
  async restoreToPointInTime(
    backupId: string,
    targetTime: number | Date,
    options?: {
      targetDatabase?: string;
      verifyOnly?: boolean;
      overwriteExisting?: boolean;
    }
  ): Promise<RestoreResponse> {
    const timestamp =
      typeof targetTime === 'number'
        ? targetTime
        : Math.floor(targetTime.getTime() / 1000);

    return this.restoreBackup(backupId, {
      point_in_time: timestamp,
      target_database: options?.targetDatabase,
      verify_only: options?.verifyOnly,
      overwrite_existing: options?.overwriteExisting,
    });
  }

  /**
   * Verify backup integrity (restore with verify_only flag)
   */
  async verifyBackup(backupId: string): Promise<RestoreResponse> {
    return this.restoreBackup(backupId, {
      verify_only: true,
    });
  }

  /**
   * Delete a backup
   * DELETE /api/v1/backup/{id}
   */
  async deleteBackup(backupId: string): Promise<void> {
    await this.client.delete(`/api/v1/backup/${backupId}`);
  }

  /**
   * Get backup schedule configuration
   * GET /api/v1/backup/schedule
   */
  async getBackupSchedule(): Promise<BackupSchedule> {
    const response = await this.client.get<BackupSchedule>(
      '/api/v1/backup/schedule'
    );
    return response.data;
  }

  /**
   * Update backup schedule configuration
   * PUT /api/v1/backup/schedule
   */
  async updateBackupSchedule(
    schedule: BackupSchedule
  ): Promise<{ success: boolean; message: string; enabled: boolean }> {
    const response = await this.client.put<{
      success: boolean;
      message: string;
      enabled: boolean;
    }>('/api/v1/backup/schedule', schedule);
    return response.data;
  }

  /**
   * Enable automated backups
   */
  async enableBackupSchedule(config?: Partial<BackupSchedule>): Promise<void> {
    const current = await this.getBackupSchedule();
    await this.updateBackupSchedule({
      ...current,
      ...config,
      enabled: true,
    });
  }

  /**
   * Disable automated backups
   */
  async disableBackupSchedule(): Promise<void> {
    const current = await this.getBackupSchedule();
    await this.updateBackupSchedule({
      ...current,
      enabled: false,
    });
  }

  // ==========================================================================
  // Flashback Operations
  // ==========================================================================

  /**
   * Execute a flashback query (AS OF timestamp/SCN)
   * POST /api/v1/flashback/query
   */
  async flashbackQuery(
    request: FlashbackQueryRequest
  ): Promise<FlashbackQueryResponse> {
    const response = await this.client.post<FlashbackQueryResponse>(
      '/api/v1/flashback/query',
      request
    );
    return response.data;
  }

  /**
   * Query table at specific timestamp
   */
  async queryAsOfTimestamp(
    table: string,
    timestamp: string | Date,
    options?: {
      columns?: string[];
      filter?: Record<string, any>;
      limit?: number;
    }
  ): Promise<FlashbackQueryResponse> {
    const timestampStr =
      typeof timestamp === 'string'
        ? timestamp
        : timestamp.toISOString();

    return this.flashbackQuery({
      table,
      timestamp: timestampStr,
      columns: options?.columns,
      filter: options?.filter,
      limit: options?.limit,
    });
  }

  /**
   * Query table at specific SCN
   */
  async queryAsOfSCN(
    table: string,
    scn: number,
    options?: {
      columns?: string[];
      filter?: Record<string, any>;
      limit?: number;
    }
  ): Promise<FlashbackQueryResponse> {
    return this.flashbackQuery({
      table,
      scn,
      columns: options?.columns,
      filter: options?.filter,
      limit: options?.limit,
    });
  }

  /**
   * Restore a table to a previous point in time
   * POST /api/v1/flashback/table
   */
  async flashbackTable(
    request: FlashbackTableRequest
  ): Promise<FlashbackTableResponse> {
    const response = await this.client.post<FlashbackTableResponse>(
      '/api/v1/flashback/table',
      request
    );
    return response.data;
  }

  /**
   * Restore table to timestamp
   */
  async restoreTableToTimestamp(
    table: string,
    timestamp: string | Date,
    options?: {
      enableTriggers?: boolean;
    }
  ): Promise<FlashbackTableResponse> {
    const timestampStr =
      typeof timestamp === 'string'
        ? timestamp
        : timestamp.toISOString();

    return this.flashbackTable({
      table,
      target_timestamp: timestampStr,
      enable_triggers: options?.enableTriggers,
    });
  }

  /**
   * Restore table to SCN
   */
  async restoreTableToSCN(
    table: string,
    scn: number,
    options?: {
      enableTriggers?: boolean;
    }
  ): Promise<FlashbackTableResponse> {
    return this.flashbackTable({
      table,
      target_scn: scn,
      enable_triggers: options?.enableTriggers,
    });
  }

  /**
   * Restore table to restore point
   */
  async restoreTableToRestorePoint(
    table: string,
    restorePoint: string,
    options?: {
      enableTriggers?: boolean;
    }
  ): Promise<FlashbackTableResponse> {
    return this.flashbackTable({
      table,
      restore_point: restorePoint,
      enable_triggers: options?.enableTriggers,
    });
  }

  /**
   * Query row versions (history) between SCNs
   * POST /api/v1/flashback/versions
   */
  async queryVersions(
    request: VersionsQueryRequest
  ): Promise<VersionsQueryResponse> {
    const response = await this.client.post<VersionsQueryResponse>(
      '/api/v1/flashback/versions',
      request
    );
    return response.data;
  }

  /**
   * Get full history of a row
   */
  async getRowHistory(
    table: string,
    primaryKey: Record<string, any>,
    options?: {
      startSCN?: number;
      endSCN?: number;
      startTimestamp?: string;
      endTimestamp?: string;
    }
  ): Promise<RowVersion[]> {
    const response = await this.queryVersions({
      table,
      primary_key: primaryKey,
      start_scn: options?.startSCN,
      end_scn: options?.endSCN,
      start_timestamp: options?.startTimestamp,
      end_timestamp: options?.endTimestamp,
    });
    return response.versions;
  }

  /**
   * Create a restore point
   * POST /api/v1/flashback/restore-points
   */
  async createRestorePoint(
    request: CreateRestorePointRequest
  ): Promise<RestorePointResponse> {
    const response = await this.client.post<RestorePointResponse>(
      '/api/v1/flashback/restore-points',
      request
    );
    return response.data;
  }

  /**
   * Create a guaranteed restore point
   */
  async createGuaranteedRestorePoint(
    name: string,
    preserveLogs: boolean = true
  ): Promise<RestorePointResponse> {
    return this.createRestorePoint({
      name,
      guaranteed: true,
      preserve_logs: preserveLogs,
    });
  }

  /**
   * Create a normal restore point
   */
  async createNormalRestorePoint(name: string): Promise<RestorePointResponse> {
    return this.createRestorePoint({
      name,
      guaranteed: false,
    });
  }

  /**
   * List all restore points
   * GET /api/v1/flashback/restore-points
   */
  async listRestorePoints(): Promise<RestorePointInfo[]> {
    const response = await this.client.get<RestorePointInfo[]>(
      '/api/v1/flashback/restore-points'
    );
    return response.data;
  }

  /**
   * Delete a restore point
   * DELETE /api/v1/flashback/restore-points/{name}
   */
  async deleteRestorePoint(name: string): Promise<void> {
    await this.client.delete(`/api/v1/flashback/restore-points/${name}`);
  }

  /**
   * Flashback entire database to a point in time
   * POST /api/v1/flashback/database
   */
  async flashbackDatabase(
    request: FlashbackDatabaseRequest
  ): Promise<FlashbackDatabaseResponse> {
    const response = await this.client.post<FlashbackDatabaseResponse>(
      '/api/v1/flashback/database',
      request
    );
    return response.data;
  }

  /**
   * Flashback database to timestamp
   */
  async flashbackDatabaseToTimestamp(
    timestamp: string | Date
  ): Promise<FlashbackDatabaseResponse> {
    const timestampStr =
      typeof timestamp === 'string'
        ? timestamp
        : timestamp.toISOString();

    return this.flashbackDatabase({
      target_timestamp: timestampStr,
    });
  }

  /**
   * Flashback database to SCN
   */
  async flashbackDatabaseToSCN(
    scn: number
  ): Promise<FlashbackDatabaseResponse> {
    return this.flashbackDatabase({
      target_scn: scn,
    });
  }

  /**
   * Flashback database to restore point
   */
  async flashbackDatabaseToRestorePoint(
    restorePoint: string
  ): Promise<FlashbackDatabaseResponse> {
    return this.flashbackDatabase({
      restore_point: restorePoint,
    });
  }

  /**
   * Get flashback statistics
   * GET /api/v1/flashback/stats
   */
  async getFlashbackStats(): Promise<FlashbackStatsResponse> {
    const response = await this.client.get<FlashbackStatsResponse>(
      '/api/v1/flashback/stats'
    );
    return response.data;
  }

  /**
   * Reverse a transaction
   * POST /api/v1/flashback/transaction
   */
  async flashbackTransaction(
    request: TransactionFlashbackRequest
  ): Promise<TransactionFlashbackResponse> {
    const response = await this.client.post<TransactionFlashbackResponse>(
      '/api/v1/flashback/transaction',
      request
    );
    return response.data;
  }

  /**
   * Reverse a transaction and all dependent transactions
   */
  async reverseTransaction(
    transactionId: string,
    cascade: boolean = false
  ): Promise<TransactionFlashbackResponse> {
    return this.flashbackTransaction({
      transaction_id: transactionId,
      cascade,
    });
  }

  /**
   * Get current System Change Number (SCN)
   * GET /api/v1/flashback/current-scn
   */
  async getCurrentSCN(): Promise<number> {
    const response = await this.client.get<number>(
      '/api/v1/flashback/current-scn'
    );
    return response.data;
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  /**
   * Wait for backup to complete
   */
  async waitForBackup(
    backupId: string,
    options?: {
      pollInterval?: number;
      timeout?: number;
      onProgress?: (backup: BackupDetails) => void;
    }
  ): Promise<BackupDetails> {
    const pollInterval = options?.pollInterval || 2000;
    const timeout = options?.timeout || 3600000; // 1 hour default
    const startTime = Date.now();

    while (true) {
      const backup = await this.getBackup(backupId);

      if (options?.onProgress) {
        options.onProgress(backup);
      }

      if (backup.status === 'completed') {
        return backup;
      }

      if (backup.status === 'failed') {
        throw new Error(
          backup.error_message || `Backup ${backupId} failed`
        );
      }

      if (backup.status === 'cancelled') {
        throw new Error(`Backup ${backupId} was cancelled`);
      }

      if (Date.now() - startTime > timeout) {
        throw new Error(`Backup ${backupId} timeout after ${timeout}ms`);
      }

      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }
  }

  /**
   * Get backup statistics
   */
  async getBackupStatistics(): Promise<{
    total: number;
    byType: Record<BackupType, number>;
    byStatus: Record<BackupStatus, number>;
    totalSize: number;
  }> {
    const list = await this.listBackups();

    const stats = {
      total: list.total_count,
      byType: { full: 0, incremental: 0 } as Record<BackupType, number>,
      byStatus: {
        in_progress: 0,
        completed: 0,
        failed: 0,
        cancelled: 0,
      } as Record<BackupStatus, number>,
      totalSize: 0,
    };

    for (const backup of list.backups) {
      stats.byType[backup.backup_type]++;
      stats.byStatus[backup.status]++;
      if (backup.size_bytes) {
        stats.totalSize += backup.size_bytes;
      }
    }

    return stats;
  }

  /**
   * Get oldest available flashback time
   */
  async getOldestFlashbackTime(): Promise<{
    scn: number;
    timestamp: number;
    retentionDays: number;
  }> {
    const stats = await this.getFlashbackStats();
    return {
      scn: stats.oldest_scn,
      timestamp: Date.now() - stats.retention_days * 24 * 60 * 60 * 1000,
      retentionDays: stats.retention_days,
    };
  }

  /**
   * Check if flashback to timestamp is possible
   */
  async canFlashbackTo(timestamp: Date | number): Promise<boolean> {
    const targetTime =
      typeof timestamp === 'number' ? timestamp : timestamp.getTime();
    const oldest = await this.getOldestFlashbackTime();
    return targetTime >= oldest.timestamp;
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a new Backup & Recovery API client
 */
export function createBackupRecoveryClient(
  config: BackupRecoveryClientConfig
): BackupRecoveryClient {
  return new BackupRecoveryClient(config);
}

// ============================================================================
// Default Export
// ============================================================================

export default BackupRecoveryClient;
