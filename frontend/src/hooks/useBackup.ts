import { useState, useEffect, useCallback, useRef } from 'react';
import { backupService } from '../services/backupService';
import type {
  Backup,
  BackupSchedule,
  RestoreProgress,
  BackupStatus,
  UUID,
} from '../types';
import type {
  BackupFilters,
  BackupProgress,
  StorageUsage,
  RestoreHistoryItem,
} from '../services/backupService';
import { getErrorMessage } from '../services/api';
import { useUIStore } from '../stores/uiStore';

// ============================================================================
// useBackups - Fetch and manage backups list
// ============================================================================

export function useBackups(filters?: BackupFilters) {
  const [backups, setBackups] = useState<Backup[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pagination, setPagination] = useState({
    total: 0,
    page: 1,
    pageSize: 20,
    totalPages: 0,
  });

  const fetchBackups = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await backupService.getBackups(filters);
      setBackups(response.data);
      setPagination({
        total: response.total,
        page: response.page,
        pageSize: response.pageSize,
        totalPages: response.totalPages,
      });
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      console.error('Failed to fetch backups:', err);
    } finally {
      setLoading(false);
    }
  }, [filters]);

  useEffect(() => {
    fetchBackups();
  }, [fetchBackups]);

  return {
    backups,
    loading,
    error,
    pagination,
    refetch: fetchBackups,
  };
}

// ============================================================================
// useBackupProgress - Poll backup progress for running backups
// ============================================================================

interface UseBackupProgressOptions {
  enabled?: boolean;
  pollInterval?: number; // milliseconds
  onComplete?: (backup: BackupProgress) => void;
  onError?: (error: string) => void;
}

export function useBackupProgress(
  backupId: UUID | null,
  options: UseBackupProgressOptions = {}
) {
  const { enabled = true, pollInterval = 2000, onComplete, onError } = options;

  const [progress, setProgress] = useState<BackupProgress | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const intervalRef = useRef<NodeJS.Timeout | null>(null);

  const fetchProgress = useCallback(async () => {
    if (!backupId || !enabled) return;

    try {
      setLoading(true);
      setError(null);
      const data = await backupService.getBackupStatus(backupId);
      setProgress(data);

      // Check if backup is complete
      if (data.status === 'completed' || data.status === 'failed') {
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }
        if (data.status === 'completed' && onComplete) {
          onComplete(data);
        }
        if (data.status === 'failed' && onError) {
          onError(data.errors?.[0] || 'Backup failed');
        }
      }
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      if (onError) {
        onError(message);
      }
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    } finally {
      setLoading(false);
    }
  }, [backupId, enabled, onComplete, onError]);

  useEffect(() => {
    if (!backupId || !enabled) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    // Initial fetch
    fetchProgress();

    // Start polling
    intervalRef.current = setInterval(fetchProgress, pollInterval);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [backupId, enabled, pollInterval, fetchProgress]);

  const cancel = useCallback(async () => {
    if (!backupId) return;

    try {
      await backupService.cancelBackup(backupId);
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      throw err;
    }
  }, [backupId]);

  return {
    progress,
    loading,
    error,
    cancel,
    refetch: fetchProgress,
  };
}

// ============================================================================
// useBackupSchedules - Fetch and manage backup schedules
// ============================================================================

export function useBackupSchedules() {
  const [schedules, setSchedules] = useState<BackupSchedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { addNotification } = useUIStore();

  const fetchSchedules = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await backupService.getSchedules();
      setSchedules(data);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      console.error('Failed to fetch schedules:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSchedules();
  }, [fetchSchedules]);

  const toggleSchedule = useCallback(
    async (id: UUID, enabled: boolean) => {
      try {
        const updated = await backupService.toggleSchedule(id, enabled);
        setSchedules((prev) =>
          prev.map((s) => (s.id === id ? updated : s))
        );
        addNotification({
          type: 'success',
          title: 'Schedule Updated',
          message: `Schedule ${enabled ? 'enabled' : 'disabled'} successfully`,
        });
      } catch (err) {
        const message = getErrorMessage(err);
        addNotification({
          type: 'error',
          title: 'Failed to Update Schedule',
          message,
        });
        throw err;
      }
    },
    [addNotification]
  );

  const deleteSchedule = useCallback(
    async (id: UUID) => {
      try {
        await backupService.deleteSchedule(id);
        setSchedules((prev) => prev.filter((s) => s.id !== id));
        addNotification({
          type: 'success',
          title: 'Schedule Deleted',
          message: 'Backup schedule deleted successfully',
        });
      } catch (err) {
        const message = getErrorMessage(err);
        addNotification({
          type: 'error',
          title: 'Failed to Delete Schedule',
          message,
        });
        throw err;
      }
    },
    [addNotification]
  );

  return {
    schedules,
    loading,
    error,
    refetch: fetchSchedules,
    toggleSchedule,
    deleteSchedule,
  };
}

// ============================================================================
// useRestoreProgress - Poll restore progress
// ============================================================================

interface UseRestoreProgressOptions {
  enabled?: boolean;
  pollInterval?: number;
  onComplete?: (restore: RestoreProgress) => void;
  onError?: (error: string) => void;
}

export function useRestoreProgress(
  restoreId: UUID | null,
  options: UseRestoreProgressOptions = {}
) {
  const { enabled = true, pollInterval = 2000, onComplete, onError } = options;

  const [progress, setProgress] = useState<RestoreProgress | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const intervalRef = useRef<NodeJS.Timeout | null>(null);

  const fetchProgress = useCallback(async () => {
    if (!restoreId || !enabled) return;

    try {
      setLoading(true);
      setError(null);
      const data = await backupService.getRestoreProgress(restoreId);
      setProgress(data);

      // Check if restore is complete
      if (data.status === 'completed' || data.status === 'failed' || data.status === 'cancelled') {
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }
        if (data.status === 'completed' && onComplete) {
          onComplete(data);
        }
        if ((data.status === 'failed' || data.status === 'cancelled') && onError) {
          onError(data.errors?.[0] || 'Restore failed');
        }
      }
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      if (onError) {
        onError(message);
      }
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    } finally {
      setLoading(false);
    }
  }, [restoreId, enabled, onComplete, onError]);

  useEffect(() => {
    if (!restoreId || !enabled) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    // Initial fetch
    fetchProgress();

    // Start polling
    intervalRef.current = setInterval(fetchProgress, pollInterval);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [restoreId, enabled, pollInterval, fetchProgress]);

  const cancel = useCallback(async () => {
    if (!restoreId) return;

    try {
      await backupService.cancelRestore(restoreId);
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      throw err;
    }
  }, [restoreId]);

  return {
    progress,
    loading,
    error,
    cancel,
    refetch: fetchProgress,
  };
}

// ============================================================================
// useStorageUsage - Fetch storage usage statistics
// ============================================================================

export function useStorageUsage() {
  const [usage, setUsage] = useState<StorageUsage | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchUsage = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await backupService.getStorageUsage();
      setUsage(data);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      console.error('Failed to fetch storage usage:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchUsage();
  }, [fetchUsage]);

  return {
    usage,
    loading,
    error,
    refetch: fetchUsage,
  };
}

// ============================================================================
// useRestoreHistory - Fetch restore history
// ============================================================================

export function useRestoreHistory(limit = 50) {
  const [history, setHistory] = useState<RestoreHistoryItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchHistory = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await backupService.getRestoreHistory(limit);
      setHistory(data);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      console.error('Failed to fetch restore history:', err);
    } finally {
      setLoading(false);
    }
  }, [limit]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  return {
    history,
    loading,
    error,
    refetch: fetchHistory,
  };
}
