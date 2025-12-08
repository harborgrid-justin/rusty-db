// ============================================================================
// Replication Status Component
// Display replication status between nodes with lag indicators
// ============================================================================

import { formatDistanceToNow } from 'date-fns';
import {
  ArrowRightIcon,
  CheckCircleIcon,
  ExclamationCircleIcon,
  XCircleIcon,
  PauseCircleIcon,
  ClockIcon,
} from '@heroicons/react/24/outline';
import type { ReplicationStatus as ReplicationStatusType, ClusterNode } from '../../types';
import clsx from 'clsx';

interface ReplicationStatusProps {
  status: ReplicationStatusType;
  sourceNode?: ClusterNode;
  targetNode?: ClusterNode;
  className?: string;
}

export function ReplicationStatus({
  status,
  sourceNode,
  targetNode,
  className = '',
}: ReplicationStatusProps) {
  function getStatusColor() {
    switch (status.status) {
      case 'streaming':
        return 'text-green-600 bg-green-50 border-green-200';
      case 'catchup':
        return 'text-blue-600 bg-blue-50 border-blue-200';
      case 'stopped':
        return 'text-gray-600 bg-gray-50 border-gray-200';
      case 'error':
        return 'text-red-600 bg-red-50 border-red-200';
      default:
        return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  }

  function getStatusIcon() {
    switch (status.status) {
      case 'streaming':
        return <CheckCircleIcon className="w-5 h-5" />;
      case 'catchup':
        return <ClockIcon className="w-5 h-5" />;
      case 'stopped':
        return <PauseCircleIcon className="w-5 h-5" />;
      case 'error':
        return <XCircleIcon className="w-5 h-5" />;
      default:
        return <ExclamationCircleIcon className="w-5 h-5" />;
    }
  }

  function getModeColor() {
    switch (status.mode) {
      case 'synchronous':
        return 'bg-purple-100 text-purple-800 border-purple-300';
      case 'asynchronous':
        return 'bg-blue-100 text-blue-800 border-blue-300';
      case 'semi_synchronous':
        return 'bg-indigo-100 text-indigo-800 border-indigo-300';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-300';
    }
  }

  function formatBytes(bytes: number): string {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  }

  function getLagSeverity() {
    const lagSeconds = status.lag / 1000;
    if (lagSeconds > 10) return 'critical';
    if (lagSeconds > 5) return 'warning';
    if (lagSeconds > 1) return 'info';
    return 'normal';
  }

  const lagSeverity = getLagSeverity();
  const lagSeconds = status.lag / 1000;

  return (
    <div className={clsx('bg-white border rounded-lg shadow-sm', className)}>
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className={clsx('flex items-center space-x-2 px-3 py-1 rounded-full border', getStatusColor())}>
              {getStatusIcon()}
              <span className="text-sm font-medium capitalize">
                {status.status}
              </span>
            </div>

            <span className={clsx('px-2.5 py-0.5 rounded-full text-xs font-medium border', getModeColor())}>
              {status.mode.replace('_', ' ').toUpperCase()}
            </span>
          </div>

          <div className="text-xs text-gray-500">
            Last sync: {formatDistanceToNow(new Date(status.lastSyncTime), { addSuffix: true })}
          </div>
        </div>
      </div>

      {/* Body */}
      <div className="p-4">
        {/* Replication Flow */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex-1">
            <div className="text-xs text-gray-500 mb-1">Source</div>
            <div className="font-medium text-gray-900">
              {sourceNode?.name || status.sourceNode}
            </div>
            {sourceNode && (
              <div className="text-xs text-gray-600 mt-0.5">
                {sourceNode.host}:{sourceNode.port}
              </div>
            )}
          </div>

          <div className="flex items-center justify-center px-4">
            <ArrowRightIcon className="w-8 h-8 text-blue-500" />
          </div>

          <div className="flex-1 text-right">
            <div className="text-xs text-gray-500 mb-1">Target</div>
            <div className="font-medium text-gray-900">
              {targetNode?.name || status.targetNode}
            </div>
            {targetNode && (
              <div className="text-xs text-gray-600 mt-0.5">
                {targetNode.host}:{targetNode.port}
              </div>
            )}
          </div>
        </div>

        {/* Lag Indicator */}
        <div
          className={clsx(
            'p-4 rounded-lg border mb-4',
            lagSeverity === 'critical' && 'bg-red-50 border-red-200',
            lagSeverity === 'warning' && 'bg-amber-50 border-amber-200',
            lagSeverity === 'info' && 'bg-blue-50 border-blue-200',
            lagSeverity === 'normal' && 'bg-green-50 border-green-200'
          )}
        >
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center space-x-2">
              <ClockIcon className="w-5 h-5 text-gray-600" />
              <span className="text-sm font-medium text-gray-900">
                Replication Lag
              </span>
            </div>
            <span
              className={clsx(
                'text-2xl font-bold',
                lagSeverity === 'critical' && 'text-red-600',
                lagSeverity === 'warning' && 'text-amber-600',
                lagSeverity === 'info' && 'text-blue-600',
                lagSeverity === 'normal' && 'text-green-600'
              )}
            >
              {lagSeconds.toFixed(2)}s
            </span>
          </div>

          {/* Lag Progress Bar */}
          <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              className={clsx(
                'h-full rounded-full transition-all',
                lagSeverity === 'critical' && 'bg-red-500',
                lagSeverity === 'warning' && 'bg-amber-500',
                lagSeverity === 'info' && 'bg-blue-500',
                lagSeverity === 'normal' && 'bg-green-500'
              )}
              style={{
                width: `${Math.min((lagSeconds / 10) * 100, 100)}%`,
              }}
            />
          </div>

          {/* Threshold Indicators */}
          <div className="flex justify-between mt-2 text-xs text-gray-600">
            <span>0s</span>
            <span className="text-blue-600">1s</span>
            <span className="text-amber-600">5s</span>
            <span className="text-red-600">10s+</span>
          </div>
        </div>

        {/* Metrics Grid */}
        <div className="grid grid-cols-3 gap-4">
          {/* Bytes Transferred */}
          <div className="bg-gray-50 rounded-lg p-3">
            <div className="text-xs text-gray-500 mb-1">Bytes Transferred</div>
            <div className="text-lg font-semibold text-gray-900">
              {formatBytes(status.bytesTransferred)}
            </div>
          </div>

          {/* Transactions */}
          <div className="bg-gray-50 rounded-lg p-3">
            <div className="text-xs text-gray-500 mb-1">Transactions</div>
            <div className="text-lg font-semibold text-gray-900">
              {status.transactionsReplicated.toLocaleString()}
            </div>
          </div>

          {/* Last Synced LSN */}
          <div className="bg-gray-50 rounded-lg p-3">
            <div className="text-xs text-gray-500 mb-1">Last Synced LSN</div>
            <div className="text-sm font-mono font-semibold text-gray-900 truncate">
              {status.lastSyncedLsn}
            </div>
          </div>
        </div>

        {/* Status Details */}
        {status.status === 'error' && (
          <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg">
            <div className="flex items-start space-x-2">
              <XCircleIcon className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
              <div>
                <div className="text-sm font-medium text-red-900">
                  Replication Error
                </div>
                <div className="text-sm text-red-700 mt-1">
                  Replication has encountered an error. Please check the logs for more details.
                </div>
              </div>
            </div>
          </div>
        )}

        {status.status === 'catchup' && (
          <div className="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-lg">
            <div className="flex items-start space-x-2">
              <ClockIcon className="w-5 h-5 text-blue-600 flex-shrink-0 mt-0.5" />
              <div>
                <div className="text-sm font-medium text-blue-900">
                  Catching Up
                </div>
                <div className="text-sm text-blue-700 mt-1">
                  The target node is catching up with the source. Replication lag is reducing.
                </div>
              </div>
            </div>
          </div>
        )}

        {status.status === 'stopped' && (
          <div className="mt-4 p-3 bg-gray-50 border border-gray-200 rounded-lg">
            <div className="flex items-start space-x-2">
              <PauseCircleIcon className="w-5 h-5 text-gray-600 flex-shrink-0 mt-0.5" />
              <div>
                <div className="text-sm font-medium text-gray-900">
                  Replication Stopped
                </div>
                <div className="text-sm text-gray-700 mt-1">
                  Replication has been manually stopped or paused.
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
