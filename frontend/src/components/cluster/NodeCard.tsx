// ============================================================================
// Node Card Component
// Display node information with health status and metrics
// ============================================================================

import { formatDistanceToNow } from 'date-fns';
import {
  ServerIcon,
  CheckCircleIcon,
  ExclamationCircleIcon,
  XCircleIcon,
  ClockIcon,
  CpuChipIcon,
  CircleStackIcon,
  ArrowPathIcon,
  TrashIcon,
  ArrowUpIcon,
  ArrowDownIcon,
} from '@heroicons/react/24/outline';
import { StarIcon } from '@heroicons/react/24/solid';
import type { ClusterNode } from '../../types';
import clsx from 'clsx';

interface NodeCardProps {
  node: ClusterNode;
  onPromote?: (nodeId: string) => void;
  onDemote?: (nodeId: string) => void;
  onRemove?: (nodeId: string) => void;
  onResync?: (nodeId: string) => void;
  onViewDetails?: (nodeId: string) => void;
  className?: string;
}

export function NodeCard({
  node,
  onPromote,
  onDemote,
  onRemove,
  onResync,
  onViewDetails,
  className = '',
}: NodeCardProps) {
  const isLeader = node.role === 'leader';
  const isHealthy = node.status === 'healthy';

  function getStatusColor() {
    switch (node.status) {
      case 'healthy':
        return 'text-green-600 bg-green-50 border-green-200';
      case 'degraded':
        return 'text-amber-600 bg-amber-50 border-amber-200';
      case 'unreachable':
        return 'text-red-600 bg-red-50 border-red-200';
      case 'shutting_down':
        return 'text-gray-600 bg-gray-50 border-gray-200';
      case 'failed':
        return 'text-red-700 bg-red-100 border-red-300';
      default:
        return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  }

  function getStatusIcon() {
    switch (node.status) {
      case 'healthy':
        return <CheckCircleIcon className="w-5 h-5" />;
      case 'degraded':
        return <ExclamationCircleIcon className="w-5 h-5" />;
      case 'unreachable':
      case 'failed':
        return <XCircleIcon className="w-5 h-5" />;
      default:
        return <ServerIcon className="w-5 h-5" />;
    }
  }

  function getRoleColor() {
    switch (node.role) {
      case 'leader':
        return 'bg-amber-100 text-amber-800 border-amber-300';
      case 'follower':
        return 'bg-blue-100 text-blue-800 border-blue-300';
      case 'candidate':
        return 'bg-purple-100 text-purple-800 border-purple-300';
      case 'observer':
        return 'bg-gray-100 text-gray-800 border-gray-300';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-300';
    }
  }

  return (
    <div
      className={clsx(
        'bg-white border-2 rounded-lg shadow-sm hover:shadow-md transition-all duration-200',
        isLeader ? 'border-amber-300' : 'border-gray-200',
        className
      )}
    >
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div
              className={clsx(
                'p-2 rounded-lg',
                isLeader ? 'bg-amber-100' : 'bg-blue-50'
              )}
            >
              {isLeader ? (
                <StarIcon className="w-6 h-6 text-amber-600" />
              ) : (
                <ServerIcon className="w-6 h-6 text-blue-600" />
              )}
            </div>

            <div>
              <h3 className="text-lg font-semibold text-gray-900 flex items-center space-x-2">
                <span>{node.name}</span>
                {isLeader && (
                  <span className="text-xs font-medium px-2 py-0.5 rounded-full bg-amber-100 text-amber-800">
                    LEADER
                  </span>
                )}
              </h3>
              <p className="text-sm text-gray-500">
                {node.host}:{node.port}
              </p>
            </div>
          </div>

          <div className={clsx('flex items-center space-x-2 px-3 py-1 rounded-full border', getStatusColor())}>
            {getStatusIcon()}
            <span className="text-sm font-medium capitalize">
              {node.status.replace('_', ' ')}
            </span>
          </div>
        </div>
      </div>

      {/* Body */}
      <div className="p-4">
        {/* Role and Version */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center space-x-2">
            <span className={clsx('px-3 py-1 rounded-full text-xs font-medium border', getRoleColor())}>
              {node.role.toUpperCase()}
            </span>
            <span className="text-xs text-gray-500">v{node.version}</span>
          </div>

          {node.region && (
            <div className="text-xs text-gray-600">
              {node.region}
              {node.zone && ` / ${node.zone}`}
            </div>
          )}
        </div>

        {/* Metrics */}
        {node.metrics && (
          <div className="grid grid-cols-2 gap-3 mb-4">
            {/* CPU */}
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="flex items-center space-x-2 mb-1">
                <CpuChipIcon className="w-4 h-4 text-gray-500" />
                <span className="text-xs font-medium text-gray-600">CPU</span>
              </div>
              <div className="flex items-baseline space-x-1">
                <span className="text-lg font-semibold text-gray-900">
                  {node.metrics.cpu.toFixed(1)}
                </span>
                <span className="text-xs text-gray-500">%</span>
              </div>
              <div className="mt-1 h-1 bg-gray-200 rounded-full overflow-hidden">
                <div
                  className={clsx(
                    'h-full rounded-full transition-all',
                    node.metrics.cpu > 80
                      ? 'bg-red-500'
                      : node.metrics.cpu > 60
                      ? 'bg-amber-500'
                      : 'bg-green-500'
                  )}
                  style={{ width: `${node.metrics.cpu}%` }}
                />
              </div>
            </div>

            {/* Memory */}
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="flex items-center space-x-2 mb-1">
                <CircleStackIcon className="w-4 h-4 text-gray-500" />
                <span className="text-xs font-medium text-gray-600">Memory</span>
              </div>
              <div className="flex items-baseline space-x-1">
                <span className="text-lg font-semibold text-gray-900">
                  {node.metrics.memory.toFixed(1)}
                </span>
                <span className="text-xs text-gray-500">%</span>
              </div>
              <div className="mt-1 h-1 bg-gray-200 rounded-full overflow-hidden">
                <div
                  className={clsx(
                    'h-full rounded-full transition-all',
                    node.metrics.memory > 80
                      ? 'bg-red-500'
                      : node.metrics.memory > 60
                      ? 'bg-amber-500'
                      : 'bg-green-500'
                  )}
                  style={{ width: `${node.metrics.memory}%` }}
                />
              </div>
            </div>

            {/* Disk */}
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="flex items-center space-x-2 mb-1">
                <CircleStackIcon className="w-4 h-4 text-gray-500" />
                <span className="text-xs font-medium text-gray-600">Disk</span>
              </div>
              <div className="flex items-baseline space-x-1">
                <span className="text-lg font-semibold text-gray-900">
                  {node.metrics.disk.toFixed(1)}
                </span>
                <span className="text-xs text-gray-500">%</span>
              </div>
              <div className="mt-1 h-1 bg-gray-200 rounded-full overflow-hidden">
                <div
                  className={clsx(
                    'h-full rounded-full transition-all',
                    node.metrics.disk > 80
                      ? 'bg-red-500'
                      : node.metrics.disk > 60
                      ? 'bg-amber-500'
                      : 'bg-green-500'
                  )}
                  style={{ width: `${node.metrics.disk}%` }}
                />
              </div>
            </div>

            {/* Connections */}
            <div className="bg-gray-50 rounded-lg p-3">
              <div className="flex items-center space-x-2 mb-1">
                <ServerIcon className="w-4 h-4 text-gray-500" />
                <span className="text-xs font-medium text-gray-600">Connections</span>
              </div>
              <div className="flex items-baseline space-x-1">
                <span className="text-lg font-semibold text-gray-900">
                  {node.metrics.connections}
                </span>
              </div>
            </div>
          </div>
        )}

        {/* Replication Lag */}
        {node.role !== 'leader' && node.metrics?.replicationLag !== undefined && (
          <div
            className={clsx(
              'mb-4 p-3 rounded-lg border',
              node.metrics.replicationLag > 5000
                ? 'bg-red-50 border-red-200'
                : node.metrics.replicationLag > 1000
                ? 'bg-amber-50 border-amber-200'
                : 'bg-green-50 border-green-200'
            )}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <ClockIcon className="w-4 h-4 text-gray-600" />
                <span className="text-sm font-medium text-gray-900">
                  Replication Lag
                </span>
              </div>
              <span
                className={clsx(
                  'text-sm font-semibold',
                  node.metrics.replicationLag > 5000
                    ? 'text-red-600'
                    : node.metrics.replicationLag > 1000
                    ? 'text-amber-600'
                    : 'text-green-600'
                )}
              >
                {(node.metrics.replicationLag / 1000).toFixed(2)}s
              </span>
            </div>
          </div>
        )}

        {/* Timestamps */}
        <div className="space-y-1 mb-4 text-xs text-gray-600">
          <div className="flex justify-between">
            <span>Started:</span>
            <span className="font-medium">
              {formatDistanceToNow(new Date(node.startTime), { addSuffix: true })}
            </span>
          </div>
          <div className="flex justify-between">
            <span>Last Heartbeat:</span>
            <span className="font-medium">
              {formatDistanceToNow(new Date(node.lastHeartbeat), {
                addSuffix: true,
              })}
            </span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex flex-wrap gap-2 pt-4 border-t border-gray-200">
          {onViewDetails && (
            <button
              onClick={() => onViewDetails(node.id)}
              className="flex-1 px-3 py-2 text-sm font-medium text-blue-600 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors"
            >
              View Details
            </button>
          )}

          {!isLeader && onPromote && (
            <button
              onClick={() => onPromote(node.id)}
              disabled={!isHealthy}
              className="flex items-center justify-center px-3 py-2 text-sm font-medium text-green-600 bg-green-50 rounded-lg hover:bg-green-100 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              title="Promote to leader"
            >
              <ArrowUpIcon className="w-4 h-4 mr-1" />
              Promote
            </button>
          )}

          {isLeader && onDemote && (
            <button
              onClick={() => onDemote(node.id)}
              className="flex items-center justify-center px-3 py-2 text-sm font-medium text-amber-600 bg-amber-50 rounded-lg hover:bg-amber-100 transition-colors"
              title="Demote from leader"
            >
              <ArrowDownIcon className="w-4 h-4 mr-1" />
              Demote
            </button>
          )}

          {!isLeader && onResync && (
            <button
              onClick={() => onResync(node.id)}
              disabled={node.status === 'unreachable'}
              className="flex items-center justify-center px-3 py-2 text-sm font-medium text-blue-600 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              title="Resync node"
            >
              <ArrowPathIcon className="w-4 h-4 mr-1" />
              Resync
            </button>
          )}

          {!isLeader && onRemove && (
            <button
              onClick={() => onRemove(node.id)}
              className="flex items-center justify-center px-3 py-2 text-sm font-medium text-red-600 bg-red-50 rounded-lg hover:bg-red-100 transition-colors"
              title="Remove node"
            >
              <TrashIcon className="w-4 h-4 mr-1" />
              Remove
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
