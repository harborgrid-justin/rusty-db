import { ConnectionPoolStats } from '../../types';
import { Card, CardHeader } from '../common/Card';
import { Badge } from '../common/Badge';
import {
  CheckCircleIcon,
  ClockIcon,
  QueueListIcon,
  ArrowPathIcon,
} from '@heroicons/react/24/outline';

// ============================================================================
// Pool Stats Card Component
// Displays connection pool statistics and health
// ============================================================================

export interface PoolStatsCardProps {
  stats: ConnectionPoolStats;
  className?: string;
}

export function PoolStatsCard({ stats, className = '' }: PoolStatsCardProps) {
  const utilizationPercentage =
    (stats.activeConnections / stats.maxConnections) * 100;

  const getHealthStatus = (): 'healthy' | 'warning' | 'critical' => {
    if (utilizationPercentage >= 90 || stats.waitingRequests > 10) {
      return 'critical';
    }
    if (utilizationPercentage >= 75 || stats.waitingRequests > 5) {
      return 'warning';
    }
    return 'healthy';
  };

  const healthStatus = getHealthStatus();
  const healthVariant = {
    healthy: 'success' as const,
    warning: 'warning' as const,
    critical: 'danger' as const,
  }[healthStatus];

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  return (
    <Card className={className}>
      <CardHeader
        title={
          <div className="flex items-center gap-3">
            <span className="text-lg font-semibold text-dark-100">
              Pool: {stats.poolId}
            </span>
            <Badge variant={healthVariant} size="sm" dot>
              {healthStatus.charAt(0).toUpperCase() + healthStatus.slice(1)}
            </Badge>
          </div>
        }
      />

      <div className="space-y-6 mt-4">
        {/* Connection Status */}
        <div>
          <h4 className="text-sm font-medium text-dark-300 mb-3">Connection Status</h4>
          <div className="grid grid-cols-2 gap-4">
            <div className="bg-dark-700/50 rounded-lg p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-dark-400">Active</span>
                <CheckCircleIcon className="w-4 h-4 text-success-500" />
              </div>
              <div className="text-2xl font-bold text-dark-100">
                {stats.activeConnections}
              </div>
              <div className="text-xs text-dark-400 mt-1">
                {utilizationPercentage.toFixed(1)}% utilization
              </div>
            </div>

            <div className="bg-dark-700/50 rounded-lg p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-dark-400">Idle</span>
                <ClockIcon className="w-4 h-4 text-dark-500" />
              </div>
              <div className="text-2xl font-bold text-dark-100">
                {stats.idleConnections}
              </div>
              <div className="text-xs text-dark-400 mt-1">
                Available for use
              </div>
            </div>

            <div className="bg-dark-700/50 rounded-lg p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-dark-400">Total</span>
                <ArrowPathIcon className="w-4 h-4 text-info-500" />
              </div>
              <div className="text-2xl font-bold text-dark-100">
                {stats.totalConnections}
              </div>
              <div className="text-xs text-dark-400 mt-1">
                {stats.minConnections} min / {stats.maxConnections} max
              </div>
            </div>

            <div className="bg-dark-700/50 rounded-lg p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-dark-400">Waiting</span>
                <QueueListIcon className="w-4 h-4 text-warning-500" />
              </div>
              <div className="text-2xl font-bold text-dark-100">
                {stats.waitingRequests}
              </div>
              <div className="text-xs text-dark-400 mt-1">
                In queue
              </div>
            </div>
          </div>
        </div>

        {/* Utilization Bar */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-dark-300">Pool Utilization</span>
            <span className="text-sm text-dark-400">
              {stats.activeConnections} / {stats.maxConnections}
            </span>
          </div>
          <div className="w-full bg-dark-700 rounded-full h-3 overflow-hidden">
            <div className="flex h-full">
              {/* Active connections */}
              <div
                className={`transition-all duration-300 ${
                  utilizationPercentage >= 90
                    ? 'bg-danger-500'
                    : utilizationPercentage >= 75
                    ? 'bg-warning-500'
                    : 'bg-success-500'
                }`}
                style={{
                  width: `${(stats.activeConnections / stats.maxConnections) * 100}%`,
                }}
              />
              {/* Idle connections */}
              <div
                className="bg-dark-600 transition-all duration-300"
                style={{
                  width: `${(stats.idleConnections / stats.maxConnections) * 100}%`,
                }}
              />
            </div>
          </div>
          <div className="flex justify-between text-xs text-dark-400 mt-2">
            <span>0</span>
            <span>Min: {stats.minConnections}</span>
            <span>Max: {stats.maxConnections}</span>
          </div>
        </div>

        {/* Performance Metrics */}
        <div>
          <h4 className="text-sm font-medium text-dark-300 mb-3">Performance</h4>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-dark-400">Avg Wait Time</span>
              <span className="text-sm font-semibold text-dark-200">
                {formatDuration(stats.avgWaitTime)}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-dark-400">Avg Connection Time</span>
              <span className="text-sm font-semibold text-dark-200">
                {formatDuration(stats.avgConnectionTime)}
              </span>
            </div>
          </div>
        </div>

        {/* Warnings */}
        {(stats.waitingRequests > 0 || utilizationPercentage >= 75) && (
          <div className="bg-warning-500/10 border border-warning-500/30 rounded-lg p-3">
            <div className="flex items-start gap-2">
              <QueueListIcon className="w-5 h-5 text-warning-500 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <div className="text-sm font-medium text-warning-400 mb-1">
                  Pool Under Pressure
                </div>
                <div className="text-xs text-warning-400/80">
                  {utilizationPercentage >= 75 &&
                    `Pool is ${utilizationPercentage.toFixed(0)}% utilized. `}
                  {stats.waitingRequests > 0 &&
                    `${stats.waitingRequests} requests waiting for connections.`}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}
