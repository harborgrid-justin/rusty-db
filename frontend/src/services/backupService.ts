import { get, post, put, del, patch, buildQueryParams } from './api';
import type {
  Backup,
  BackupSchedule,
  RestoreRequest,
  RestoreProgress,
  BackupType,
  BackupStatus,
  PaginatedResponse,
  PaginationParams,
  UUID,
  Timestamp,
} from '../types';

// ============================================================================
// Backup Service Types
// ============================================================================

export interface BackupFilters extends Partial<PaginationParams> {
  status?: BackupStatus;
  type?: BackupType;
  database?: string;
  startDate?: Timestamp;
  endDate?: Timestamp;
  [key: string]: unknown;
}

export interface CreateBackupConfig {
  name?: string;
  type: BackupType;
  database?: string;
  compression?: boolean;
  encrypted?: boolean;
  retentionDays?: number;
  metadata?: Record<string, unknown>;
}

export interface CreateScheduleConfig {
  name: string;
  type: BackupType;
  database?: string;
  schedule: string; // cron expression
  retentionDays: number;
  compression?: boolean;
  encrypted?: boolean;
  isEnabled?: boolean;
}

export interface UpdateScheduleConfig extends Partial<CreateScheduleConfig> {}

export interface BackupProgress {
  id: UUID;
  status: BackupStatus;
  progress: number; // percentage 0-100
  currentPhase: string;
  bytesProcessed: number;
  totalBytes?: number;
  estimatedCompletion?: Timestamp;
  startTime: Timestamp;
  errors?: string[];
}

export interface StorageUsage {
  totalBackups: number;
  totalSize: number;
  usedSpace: number;
  availableSpace: number;
  oldestBackup?: Timestamp;
  newestBackup?: Timestamp;
  byType: Record<BackupType, { count: number; size: number }>;
  byStatus: Record<BackupStatus, number>;
}

export interface RestoreHistoryItem {
  id: UUID;
  backupId: UUID;
  backupName: string;
  status: 'completed' | 'failed' | 'cancelled';
  targetDatabase: string;
  startTime: Timestamp;
  endTime?: Timestamp;
  duration?: number;
  restoredSize: number;
  errorMessage?: string;
}

// ============================================================================
// Backup Service
// ============================================================================

export const backupService = {
  /**
   * List backups with optional filters
   */
  async getBackups(filters?: BackupFilters): Promise<PaginatedResponse<Backup>> {
    const queryParams = filters ? buildQueryParams(filters) : '';
    const response = await get<PaginatedResponse<Backup>>(`/backups${queryParams}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch backups');
    }

    return response.data;
  },

  /**
   * Get a single backup by ID
   */
  async getBackup(id: UUID): Promise<Backup> {
    const response = await get<Backup>(`/backups/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch backup');
    }

    return response.data;
  },

  /**
   * Create a new backup
   */
  async createBackup(config: CreateBackupConfig): Promise<Backup> {
    const response = await post<Backup>('/backups', config);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create backup');
    }

    return response.data;
  },

  /**
   * Get backup progress/status
   */
  async getBackupStatus(id: UUID): Promise<BackupProgress> {
    const response = await get<BackupProgress>(`/backups/${id}/progress`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch backup status');
    }

    return response.data;
  },

  /**
   * Delete a backup
   */
  async deleteBackup(id: UUID): Promise<void> {
    const response = await del(`/backups/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete backup');
    }
  },

  /**
   * Get download URL for a backup
   */
  async downloadBackup(id: UUID): Promise<string> {
    const response = await get<{ url: string }>(`/backups/${id}/download`);

    if (!response.success || !response.data?.url) {
      throw new Error(response.error?.message || 'Failed to get download URL');
    }

    return response.data.url;
  },

  /**
   * Cancel a running backup
   */
  async cancelBackup(id: UUID): Promise<void> {
    const response = await post(`/backups/${id}/cancel`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to cancel backup');
    }
  },

  /**
   * Get storage usage statistics
   */
  async getStorageUsage(): Promise<StorageUsage> {
    const response = await get<StorageUsage>('/backups/storage');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch storage usage');
    }

    return response.data;
  },

  // ============================================================================
  // Backup Schedules
  // ============================================================================

  /**
   * List all backup schedules
   */
  async getSchedules(): Promise<BackupSchedule[]> {
    const response = await get<BackupSchedule[]>('/backups/schedules');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch schedules');
    }

    return response.data;
  },

  /**
   * Get a single schedule by ID
   */
  async getSchedule(id: UUID): Promise<BackupSchedule> {
    const response = await get<BackupSchedule>(`/backups/schedules/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch schedule');
    }

    return response.data;
  },

  /**
   * Create a new backup schedule
   */
  async createSchedule(config: CreateScheduleConfig): Promise<BackupSchedule> {
    const response = await post<BackupSchedule>('/backups/schedules', config);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create schedule');
    }

    return response.data;
  },

  /**
   * Update an existing schedule
   */
  async updateSchedule(id: UUID, config: UpdateScheduleConfig): Promise<BackupSchedule> {
    const response = await put<BackupSchedule>(`/backups/schedules/${id}`, config);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update schedule');
    }

    return response.data;
  },

  /**
   * Delete a backup schedule
   */
  async deleteSchedule(id: UUID): Promise<void> {
    const response = await del(`/backups/schedules/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete schedule');
    }
  },

  /**
   * Toggle schedule enabled/disabled
   */
  async toggleSchedule(id: UUID, enabled: boolean): Promise<BackupSchedule> {
    const response = await patch<BackupSchedule>(`/backups/schedules/${id}`, {
      isEnabled: enabled,
    });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to toggle schedule');
    }

    return response.data;
  },

  /**
   * Manually trigger a scheduled backup
   */
  async triggerSchedule(id: UUID): Promise<Backup> {
    const response = await post<Backup>(`/backups/schedules/${id}/trigger`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to trigger schedule');
    }

    return response.data;
  },

  // ============================================================================
  // Restore Operations
  // ============================================================================

  /**
   * Start a restore operation
   */
  async startRestore(request: RestoreRequest): Promise<RestoreProgress> {
    const response = await post<RestoreProgress>('/backups/restore', request);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to start restore');
    }

    return response.data;
  },

  /**
   * Get restore progress
   */
  async getRestoreProgress(id: UUID): Promise<RestoreProgress> {
    const response = await get<RestoreProgress>(`/backups/restore/${id}/progress`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch restore progress');
    }

    return response.data;
  },

  /**
   * Cancel a running restore
   */
  async cancelRestore(id: UUID): Promise<void> {
    const response = await post(`/backups/restore/${id}/cancel`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to cancel restore');
    }
  },

  /**
   * Get restore history
   */
  async getRestoreHistory(limit = 50): Promise<RestoreHistoryItem[]> {
    const response = await get<RestoreHistoryItem[]>(
      `/backups/restore/history?limit=${limit}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch restore history');
    }

    return response.data;
  },

  /**
   * Get available recovery points for point-in-time recovery
   */
  async getRecoveryPoints(
    backupId?: UUID,
    startTime?: Timestamp,
    endTime?: Timestamp
  ): Promise<Timestamp[]> {
    const params = buildQueryParams({
      backupId,
      startTime,
      endTime,
    });

    const response = await get<Timestamp[]>(`/backups/recovery-points${params}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch recovery points');
    }

    return response.data;
  },

  /**
   * Verify backup integrity
   */
  async verifyBackup(id: UUID): Promise<{ valid: boolean; errors?: string[] }> {
    const response = await post<{ valid: boolean; errors?: string[] }>(
      `/backups/${id}/verify`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to verify backup');
    }

    return response.data;
  },
};
