import { useState, useCallback, useEffect } from 'react';
import { motion } from 'framer-motion';
import {
  ArrowPathIcon,
  ClockIcon,
  CheckCircleIcon,
  XCircleIcon,
  ExclamationCircleIcon,
} from '@heroicons/react/24/outline';
import { RestoreWizard } from '../components/backup/RestoreWizard';
import { useBackups, useRestoreProgress, useRestoreHistory } from '../hooks/useBackup';
import { backupService } from '../services/backupService';
import type { RestoreRequest } from '../types';
import { useUIStore } from '../stores/uiStore';
import { formatBytes, formatDate, formatDuration, formatRelativeTime } from '../utils/format';
import clsx from 'clsx';

// ============================================================================
// Restore History Item Component
// ============================================================================

interface RestoreHistoryItemProps {
  item: {
    id: string;
    backupId: string;
    backupName: string;
    status: 'completed' | 'failed' | 'cancelled';
    targetDatabase: string;
    startTime: string;
    endTime?: string;
    duration?: number;
    restoredSize: number;
    errorMessage?: string;
  };
}

function RestoreHistoryItem({ item }: RestoreHistoryItemProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="card p-4"
    >
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <h4 className="font-medium text-dark-100">{item.backupName}</h4>
            <span
              className={clsx(
                'badge',
                item.status === 'completed' && 'badge-success',
                item.status === 'failed' && 'badge-danger',
                item.status === 'cancelled' && 'badge-secondary'
              )}
            >
              {item.status === 'completed' && <CheckCircleIcon className="w-3.5 h-3.5" />}
              {item.status === 'failed' && <XCircleIcon className="w-3.5 h-3.5" />}
              {item.status === 'cancelled' && <ExclamationCircleIcon className="w-3.5 h-3.5" />}
              {item.status}
            </span>
          </div>

          <div className="space-y-1 text-sm text-dark-400">
            <div>
              Target: <span className="text-dark-300">{item.targetDatabase}</span>
            </div>
            <div>
              Size: <span className="text-dark-300">{formatBytes(item.restoredSize)}</span>
              {item.duration && (
                <>
                  {' • '}
                  Duration: <span className="text-dark-300">{formatDuration(item.duration)}</span>
                </>
              )}
            </div>
            {item.errorMessage && (
              <div className="text-danger-400 mt-2">
                Error: {item.errorMessage}
              </div>
            )}
          </div>
        </div>

        <div className="text-right">
          <div className="text-xs text-dark-400">
            {formatRelativeTime(item.startTime)}
          </div>
          <div className="text-xs text-dark-500 mt-0.5">
            {formatDate(item.startTime)}
          </div>
        </div>
      </div>
    </motion.div>
  );
}

// ============================================================================
// Restore Page Component
// ============================================================================

export default function RestorePage() {
  const [showWizard, setShowWizard] = useState(false);
  const [restoreId, setRestoreId] = useState<string | null>(null);
  const [initialBackupId, setInitialBackupId] = useState<string | undefined>();

  const { backups, loading: backupsLoading } = useBackups({ status: 'completed' });
  const { history, loading: historyLoading, refetch: refetchHistory } =
    useRestoreHistory(10);
  const { addNotification } = useUIStore();

  // Check URL params for backup ID
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const backupId = params.get('backupId');
    if (backupId) {
      setInitialBackupId(backupId);
      setShowWizard(true);
    }
  }, []);

  // Track restore progress
  const { progress: restoreProgress } = useRestoreProgress(restoreId, {
    enabled: restoreId !== null,
    onComplete: () => {
      addNotification({
        type: 'success',
        title: 'Restore Completed',
        message: 'The database has been restored successfully',
      });
      setRestoreId(null);
      setShowWizard(false);
      refetchHistory();
    },
    onError: (error) => {
      addNotification({
        type: 'error',
        title: 'Restore Failed',
        message: error,
      });
      setRestoreId(null);
      refetchHistory();
    },
  });

  const handleStartRestore = useCallback(
    async (request: RestoreRequest) => {
      try {
        const progress = await backupService.startRestore(request);
        setRestoreId(progress.id);
        addNotification({
          type: 'info',
          title: 'Restore Started',
          message: 'Database restore operation has been initiated',
        });
      } catch (error) {
        console.error('Failed to start restore:', error);
        throw error;
      }
    },
    [addNotification]
  );

  const handleCancel = useCallback(() => {
    setShowWizard(false);
    setInitialBackupId(undefined);
  }, []);

  const isRestoring = restoreProgress && restoreProgress.status === 'running';

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100">Restore Database</h1>
          <p className="text-dark-400 mt-1">
            Restore your database from a backup
          </p>
        </div>

        {!showWizard && !isRestoring && (
          <button
            onClick={() => setShowWizard(true)}
            className="btn-primary"
          >
            <ArrowPathIcon className="w-5 h-5" />
            Start Restore
          </button>
        )}
      </div>

      {/* Warning Banner */}
      <div className="p-4 bg-warning-500/10 border border-warning-500/30 rounded-lg">
        <div className="flex items-start gap-3">
          <ExclamationCircleIcon className="w-5 h-5 text-warning-400 flex-shrink-0 mt-0.5" />
          <div>
            <h4 className="text-sm font-medium text-warning-300 mb-1">
              Important Information
            </h4>
            <p className="text-sm text-warning-200">
              Restoring a database will replace existing data. Make sure you have a
              recent backup before proceeding. Consider testing restores in a staging
              environment first.
            </p>
          </div>
        </div>
      </div>

      {/* Restore Wizard or Progress */}
      {showWizard && !isRestoring ? (
        <div className="card p-6">
          <RestoreWizard
            backups={backups}
            databases={[]} // TODO: Fetch available databases
            onRestore={handleStartRestore}
            onCancel={handleCancel}
            initialBackupId={initialBackupId}
            loading={false}
          />
        </div>
      ) : isRestoring && restoreProgress ? (
        <div className="space-y-6">
          <div className="card p-6">
            <h2 className="text-xl font-semibold text-dark-100 mb-6">
              Restore in Progress
            </h2>

            {/* Progress Bar */}
            <div className="mb-6">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-dark-200">
                  {restoreProgress.currentPhase}
                </span>
                <span className="text-sm text-dark-400">
                  {restoreProgress.progress.toFixed(1)}%
                </span>
              </div>
              <div className="w-full h-2 bg-dark-700 rounded-full overflow-hidden">
                <motion.div
                  className="h-full bg-rusty-500 rounded-full"
                  initial={{ width: 0 }}
                  animate={{ width: `${restoreProgress.progress}%` }}
                  transition={{ duration: 0.5 }}
                />
              </div>
            </div>

            {/* Stats */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
                <div className="text-xs text-dark-400 mb-1">Restored</div>
                <div className="text-lg font-semibold text-dark-100">
                  {formatBytes(restoreProgress.bytesRestored)}
                </div>
                {restoreProgress.totalBytes > 0 && (
                  <div className="text-xs text-dark-400 mt-0.5">
                    of {formatBytes(restoreProgress.totalBytes)}
                  </div>
                )}
              </div>

              <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
                <div className="text-xs text-dark-400 mb-1">Tables</div>
                <div className="text-lg font-semibold text-dark-100">
                  {restoreProgress.tablesRestored}
                </div>
                {restoreProgress.totalTables > 0 && (
                  <div className="text-xs text-dark-400 mt-0.5">
                    of {restoreProgress.totalTables}
                  </div>
                )}
              </div>

              <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
                <div className="text-xs text-dark-400 mb-1">Progress</div>
                <div className="text-lg font-semibold text-dark-100">
                  {restoreProgress.progress.toFixed(1)}%
                </div>
              </div>

              <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
                <div className="text-xs text-dark-400 mb-1">Status</div>
                <div className="text-sm font-medium text-rusty-400 capitalize">
                  {restoreProgress.status}
                </div>
              </div>
            </div>

            {/* Errors */}
            {restoreProgress.errors && restoreProgress.errors.length > 0 && (
              <div className="mt-6 p-4 bg-danger-500/10 border border-danger-500/30 rounded-lg">
                <h4 className="text-sm font-medium text-danger-300 mb-2">
                  Errors Occurred
                </h4>
                <ul className="space-y-1">
                  {restoreProgress.errors.map((error, index) => (
                    <li key={index} className="text-sm text-danger-200">
                      {error}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </div>
      ) : null}

      {/* Restore History */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-dark-100">
            Recent Restores
          </h2>
          <button
            onClick={() => refetchHistory()}
            className="btn-ghost btn-sm"
          >
            <ArrowPathIcon className="w-4 h-4" />
            Refresh
          </button>
        </div>

        {historyLoading ? (
          <div className="card p-8 text-center">
            <div className="inline-block w-6 h-6 border-4 border-dark-600 border-t-rusty-500 rounded-full animate-spin" />
            <p className="mt-3 text-sm text-dark-400">Loading history...</p>
          </div>
        ) : history.length === 0 ? (
          <div className="card p-8 text-center">
            <ClockIcon className="w-12 h-12 text-dark-600 mx-auto" />
            <h3 className="mt-4 text-lg font-medium text-dark-300">
              No restore history
            </h3>
            <p className="mt-2 text-dark-400">
              Your restore operations will appear here
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {history.map((item) => (
              <RestoreHistoryItem key={item.id} item={item} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
