import { motion } from 'framer-motion';
import {
  CheckCircleIcon,
  XCircleIcon,
  ClockIcon,
  XMarkIcon,
  ExclamationTriangleIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { BackupProgress as BackupProgressType } from '../../services/backupService';
import { formatBytes, formatDuration, formatRelativeTime } from '../../utils/format';

// ============================================================================
// Types
// ============================================================================

interface BackupProgressProps {
  progress: BackupProgressType;
  onCancel?: () => void;
  showCancel?: boolean;
  compact?: boolean;
}

// ============================================================================
// Progress Bar Component
// ============================================================================

function ProgressBar({ progress, status }: { progress: number; status: string }) {
  const getColor = () => {
    if (status === 'completed') return 'bg-success-500';
    if (status === 'failed') return 'bg-danger-500';
    return 'bg-rusty-500';
  };

  return (
    <div className="w-full h-2 bg-dark-700 rounded-full overflow-hidden">
      <motion.div
        className={clsx('h-full rounded-full transition-colors', getColor())}
        initial={{ width: 0 }}
        animate={{ width: `${Math.min(100, Math.max(0, progress))}%` }}
        transition={{ duration: 0.5, ease: 'easeOut' }}
      />
    </div>
  );
}

// ============================================================================
// Status Icon Component
// ============================================================================

function StatusIcon({ status }: { status: string }) {
  switch (status) {
    case 'completed':
      return <CheckCircleIcon className="w-6 h-6 text-success-500" />;
    case 'failed':
      return <XCircleIcon className="w-6 h-6 text-danger-500" />;
    case 'pending':
      return <ClockIcon className="w-6 h-6 text-warning-500" />;
    case 'running':
      return (
        <div className="relative">
          <div className="w-6 h-6 border-4 border-rusty-500/30 border-t-rusty-500 rounded-full animate-spin" />
        </div>
      );
    default:
      return <ClockIcon className="w-6 h-6 text-dark-400" />;
  }
}

// ============================================================================
// Compact Progress Component
// ============================================================================

function CompactProgress({ progress, onCancel, showCancel }: BackupProgressProps) {
  if (!progress) return null;

  return (
    <div className="flex items-center gap-4 p-4 bg-dark-750 border border-dark-600 rounded-lg">
      <StatusIcon status={progress.status} />

      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium text-dark-200">
            {progress.currentPhase}
          </span>
          <span className="text-sm text-dark-400">
            {progress.progress.toFixed(1)}%
          </span>
        </div>
        <ProgressBar progress={progress.progress} status={progress.status} />
      </div>

      {showCancel && progress.status === 'running' && onCancel && (
        <button
          onClick={onCancel}
          className="btn-ghost btn-sm text-danger-400 hover:text-danger-300"
          title="Cancel backup"
        >
          <XMarkIcon className="w-5 h-5" />
        </button>
      )}
    </div>
  );
}

// ============================================================================
// Full Progress Component
// ============================================================================

export function BackupProgress({
  progress,
  onCancel,
  showCancel = true,
  compact = false,
}: BackupProgressProps) {
  if (!progress) return null;

  if (compact) {
    return <CompactProgress progress={progress} onCancel={onCancel} showCancel={showCancel} />;
  }

  const isRunning = progress.status === 'running';
  const isCompleted = progress.status === 'completed';
  const isFailed = progress.status === 'failed';
  const estimatedRemaining = progress.estimatedCompletion
    ? new Date(progress.estimatedCompletion).getTime() - Date.now()
    : null;

  return (
    <div className="card">
      <div className="p-6">
        {/* Header */}
        <div className="flex items-start justify-between mb-6">
          <div className="flex items-center gap-4">
            <StatusIcon status={progress.status} />
            <div>
              <h3 className="text-lg font-semibold text-dark-100">
                {isCompleted && 'Backup Completed'}
                {isFailed && 'Backup Failed'}
                {isRunning && 'Backup in Progress'}
                {progress.status === 'pending' && 'Backup Pending'}
              </h3>
              <p className="text-sm text-dark-400 mt-0.5">
                Started {formatRelativeTime(progress.startTime)}
              </p>
            </div>
          </div>

          {showCancel && isRunning && onCancel && (
            <button
              onClick={onCancel}
              className="btn-danger btn-sm"
              title="Cancel backup"
            >
              <XMarkIcon className="w-4 h-4" />
              Cancel
            </button>
          )}
        </div>

        {/* Progress Bar */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-dark-200">
              {progress.currentPhase}
            </span>
            <span className="text-sm text-dark-400">
              {progress.progress.toFixed(1)}%
            </span>
          </div>
          <ProgressBar progress={progress.progress} status={progress.status} />
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          {/* Bytes Processed */}
          <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
            <div className="text-xs text-dark-400 mb-1">Processed</div>
            <div className="text-lg font-semibold text-dark-100">
              {formatBytes(progress.bytesProcessed)}
            </div>
            {progress.totalBytes && (
              <div className="text-xs text-dark-400 mt-0.5">
                of {formatBytes(progress.totalBytes)}
              </div>
            )}
          </div>

          {/* Estimated Time */}
          {estimatedRemaining !== null && estimatedRemaining > 0 && (
            <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
              <div className="text-xs text-dark-400 mb-1">Est. Remaining</div>
              <div className="text-lg font-semibold text-dark-100">
                {formatDuration(estimatedRemaining)}
              </div>
            </div>
          )}

          {/* Progress Percentage */}
          <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
            <div className="text-xs text-dark-400 mb-1">Progress</div>
            <div className="text-lg font-semibold text-dark-100">
              {progress.progress.toFixed(1)}%
            </div>
          </div>

          {/* Status */}
          <div className="p-3 bg-dark-750 rounded-lg border border-dark-600">
            <div className="text-xs text-dark-400 mb-1">Status</div>
            <div className={clsx(
              'text-sm font-medium capitalize',
              isCompleted && 'text-success-400',
              isFailed && 'text-danger-400',
              isRunning && 'text-rusty-400',
              progress.status === 'pending' && 'text-warning-400'
            )}>
              {progress.status}
            </div>
          </div>
        </div>

        {/* Errors */}
        {progress.errors && progress.errors.length > 0 && (
          <div className="p-4 bg-danger-500/10 border border-danger-500/30 rounded-lg">
            <div className="flex items-start gap-3">
              <ExclamationTriangleIcon className="w-5 h-5 text-danger-400 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <h4 className="text-sm font-medium text-danger-300 mb-2">
                  Errors Occurred
                </h4>
                <ul className="space-y-1">
                  {progress.errors.map((error, index) => (
                    <li key={index} className="text-sm text-danger-200">
                      {error}
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
        )}

        {/* Timeline/Phases */}
        {isRunning && (
          <div className="mt-6 pt-6 border-t border-dark-700">
            <h4 className="text-sm font-medium text-dark-200 mb-3">
              Current Phase
            </h4>
            <div className="flex items-center gap-3">
              <div className="w-2 h-2 bg-rusty-500 rounded-full animate-pulse" />
              <span className="text-sm text-dark-300">{progress.currentPhase}</span>
            </div>
          </div>
        )}

        {/* Completion Message */}
        {isCompleted && (
          <div className="mt-6 p-4 bg-success-500/10 border border-success-500/30 rounded-lg">
            <div className="flex items-center gap-3">
              <CheckCircleIcon className="w-5 h-5 text-success-400" />
              <div>
                <p className="text-sm font-medium text-success-300">
                  Backup completed successfully
                </p>
                <p className="text-xs text-success-200 mt-0.5">
                  {formatBytes(progress.bytesProcessed)} backed up
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
