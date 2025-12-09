import { useState, useCallback, useEffect } from 'react';
import { motion } from 'framer-motion';
import {
  CloudArrowUpIcon,
  FunnelIcon,
  ArrowPathIcon,
  ChartBarIcon,
  ExclamationTriangleIcon,
} from '@heroicons/react/24/outline';
import { BackupList } from '../components/backup/BackupList';
import { CreateBackupModal } from '../components/backup/CreateBackupModal';
import { BackupProgress } from '../components/backup/BackupProgress';
import { useBackups, useBackupProgress, useStorageUsage } from '../hooks/useBackup';
import { backupService } from '../services/backupService';
import { configService } from '../services/configService';
import type { CreateBackupConfig } from '../services/backupService';
import type { Backup, BackupType, BackupStatus } from '../types';
import { useUIStore } from '../stores/uiStore';
import { formatBytes, formatPercent } from '../utils/format';
import clsx from 'clsx';

// ============================================================================
// Backup Page Component
// ============================================================================

export default function BackupPage() {
  const [filters, setFilters] = useState<{
    status?: BackupStatus;
    type?: BackupType;
  }>({});
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showFilters, setShowFilters] = useState(false);
  const [runningBackupId, setRunningBackupId] = useState<string | null>(null);
  const [databases, setDatabases] = useState<string[]>([]);

  const { backups, loading, error, refetch } = useBackups(filters);
  const { usage, loading: usageLoading } = useStorageUsage();
  const { addNotification, showConfirmDialog, hideConfirmDialog } = useUIStore();

  // Fetch available databases
  useEffect(() => {
    const fetchDatabases = async () => {
      try {
        const response = await configService.getAvailableDatabases();
        if (response.data) {
          setDatabases(response.data);
        }
      } catch (error) {
        console.error('Failed to fetch databases:', error);
      }
    };
    fetchDatabases();
  }, []);

  // Track running backup progress
  const { progress: backupProgress } = useBackupProgress(runningBackupId, {
    enabled: runningBackupId !== null,
    onComplete: () => {
      addNotification({
        type: 'success',
        title: 'Backup Completed',
        message: 'The backup operation completed successfully',
      });
      setRunningBackupId(null);
      refetch();
    },
    onError: (error) => {
      addNotification({
        type: 'error',
        title: 'Backup Failed',
        message: error,
      });
      setRunningBackupId(null);
      refetch();
    },
  });

  const handleCreateBackup = useCallback(
    async (config: CreateBackupConfig) => {
      try {
        const backup = await backupService.createBackup(config);
        setRunningBackupId(backup.id);
        addNotification({
          type: 'success',
          title: 'Backup Started',
          message: `Backup "${config.name}" has been started`,
        });
        refetch();
      } catch (error) {
        console.error('Failed to create backup:', error);
        throw error;
      }
    },
    [addNotification, refetch]
  );

  const handleDownload = useCallback(
    async (backup: Backup) => {
      try {
        const url = await backupService.downloadBackup(backup.id);
        window.open(url, '_blank');
        addNotification({
          type: 'info',
          title: 'Download Started',
          message: `Downloading backup "${backup.name}"`,
        });
      } catch (error) {
        addNotification({
          type: 'error',
          title: 'Download Failed',
          message: 'Failed to download backup',
        });
      }
    },
    [addNotification]
  );

  const handleDelete = useCallback(
    (backup: Backup) => {
      showConfirmDialog({
        title: 'Delete Backup',
        message: `Are you sure you want to delete "${backup.name}"? This action cannot be undone.`,
        confirmLabel: 'Delete',
        cancelLabel: 'Cancel',
        variant: 'danger',
        onConfirm: async () => {
          try {
            await backupService.deleteBackup(backup.id);
            addNotification({
              type: 'success',
              title: 'Backup Deleted',
              message: `Backup "${backup.name}" has been deleted`,
            });
            refetch();
          } catch (error) {
            addNotification({
              type: 'error',
              title: 'Delete Failed',
              message: 'Failed to delete backup',
            });
          } finally {
            hideConfirmDialog();
          }
        },
        onCancel: () => {
          hideConfirmDialog();
        },
      });
    },
    [showConfirmDialog, hideConfirmDialog, addNotification, refetch]
  );

  const handleVerify = useCallback(
    async (backup: Backup) => {
      try {
        addNotification({
          type: 'info',
          title: 'Verification Started',
          message: `Verifying backup "${backup.name}"...`,
        });

        const result = await backupService.verifyBackup(backup.id);

        if (result.valid) {
          addNotification({
            type: 'success',
            title: 'Verification Successful',
            message: `Backup "${backup.name}" is valid and can be restored`,
          });
        } else {
          addNotification({
            type: 'error',
            title: 'Verification Failed',
            message:
              result.errors?.join(', ') || 'Backup integrity check failed',
          });
        }
      } catch (error) {
        addNotification({
          type: 'error',
          title: 'Verification Failed',
          message: 'Failed to verify backup',
        });
      }
    },
    [addNotification]
  );

  const handleRestore = useCallback(
    (backup: Backup) => {
      // Navigate to restore page with this backup selected
      window.location.href = `/backup/restore?backupId=${backup.id}`;
    },
    []
  );

  const storageUsagePercent = usage
    ? (usage.usedSpace / (usage.usedSpace + usage.availableSpace)) * 100
    : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100">Backups</h1>
          <p className="text-dark-400 mt-1">
            Manage and monitor database backups
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowFilters(!showFilters)}
            className={clsx(
              'btn-secondary',
              showFilters && 'bg-rusty-500/20 border-rusty-500'
            )}
          >
            <FunnelIcon className="w-4 h-4" />
            Filters
          </button>
          <button onClick={() => refetch()} className="btn-secondary">
            <ArrowPathIcon className="w-4 h-4" />
            Refresh
          </button>
          <button
            onClick={() => setShowCreateModal(true)}
            className="btn-primary"
          >
            <CloudArrowUpIcon className="w-5 h-5" />
            Create Backup
          </button>
        </div>
      </div>

      {/* Storage Usage Stats */}
      {!usageLoading && usage && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="card p-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
                <ChartBarIcon className="w-5 h-5 text-rusty-400" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs text-dark-400">Total Backups</div>
                <div className="text-xl font-semibold text-dark-100">
                  {usage.totalBackups}
                </div>
              </div>
            </div>
          </div>

          <div className="card p-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-blue-500/20 rounded-lg flex items-center justify-center">
                <CloudArrowUpIcon className="w-5 h-5 text-blue-400" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs text-dark-400">Total Size</div>
                <div className="text-xl font-semibold text-dark-100">
                  {formatBytes(usage.totalSize)}
                </div>
              </div>
            </div>
          </div>

          <div className="card p-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-purple-500/20 rounded-lg flex items-center justify-center">
                <ChartBarIcon className="w-5 h-5 text-purple-400" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs text-dark-400">Used Space</div>
                <div className="text-xl font-semibold text-dark-100">
                  {formatBytes(usage.usedSpace)}
                </div>
              </div>
            </div>
          </div>

          <div className="card p-4">
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-2">
                <div className="text-xs text-dark-400">Storage Usage</div>
                <div className="text-xs font-medium text-dark-100">
                  {formatPercent(storageUsagePercent)}
                </div>
              </div>
              <div className="h-2 bg-dark-700 rounded-full overflow-hidden">
                <motion.div
                  className={clsx(
                    'h-full rounded-full',
                    storageUsagePercent > 90
                      ? 'bg-danger-500'
                      : storageUsagePercent > 75
                      ? 'bg-warning-500'
                      : 'bg-success-500'
                  )}
                  initial={{ width: 0 }}
                  animate={{ width: `${storageUsagePercent}%` }}
                  transition={{ duration: 1, ease: 'easeOut' }}
                />
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Storage Warning */}
      {usage && storageUsagePercent > 90 && (
        <div className="p-4 bg-danger-500/10 border border-danger-500/30 rounded-lg">
          <div className="flex items-start gap-3">
            <ExclamationTriangleIcon className="w-5 h-5 text-danger-400 flex-shrink-0" />
            <div>
              <h4 className="text-sm font-medium text-danger-300 mb-1">
                Storage Almost Full
              </h4>
              <p className="text-sm text-danger-200">
                You are using {formatPercent(storageUsagePercent)} of available
                backup storage. Consider deleting old backups or increasing
                storage capacity.
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Filters */}
      {showFilters && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: 'auto' }}
          exit={{ opacity: 0, height: 0 }}
          className="card p-4"
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-dark-200 mb-2">
                Status
              </label>
              <select
                value={filters.status || ''}
                onChange={(e) =>
                  setFilters({
                    ...filters,
                    status: e.target.value as BackupStatus | undefined,
                  })
                }
                className="input w-full"
              >
                <option value="">All Statuses</option>
                <option value="completed">Completed</option>
                <option value="running">Running</option>
                <option value="pending">Pending</option>
                <option value="failed">Failed</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-dark-200 mb-2">
                Type
              </label>
              <select
                value={filters.type || ''}
                onChange={(e) =>
                  setFilters({
                    ...filters,
                    type: e.target.value as BackupType | undefined,
                  })
                }
                className="input w-full"
              >
                <option value="">All Types</option>
                <option value="full">Full</option>
                <option value="incremental">Incremental</option>
                <option value="differential">Differential</option>
                <option value="logical">Logical</option>
                <option value="physical">Physical</option>
              </select>
            </div>
          </div>
        </motion.div>
      )}

      {/* Running Backup Progress */}
      {backupProgress && (
        <BackupProgress
          progress={backupProgress}
          onCancel={async () => {
            if (runningBackupId) {
              await backupService.cancelBackup(runningBackupId);
              setRunningBackupId(null);
              refetch();
            }
          }}
        />
      )}

      {/* Error State */}
      {error && (
        <div className="card p-4 bg-danger-500/10 border border-danger-500/30">
          <p className="text-danger-300">{error}</p>
        </div>
      )}

      {/* Backup List */}
      <BackupList
        backups={backups}
        loading={loading}
        onDownload={handleDownload}
        onDelete={handleDelete}
        onRestore={handleRestore}
        onVerify={handleVerify}
      />

      {/* Create Backup Modal */}
      <CreateBackupModal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onCreate={handleCreateBackup}
        databases={databases}
      />
    </div>
  );
}
