import { ResourceGroup } from '../../types';
import { Card, CardHeader } from '../common/Card';
import { Badge } from '../common/Badge';
import { Button } from '../common/Button';
import { Tooltip } from '../common/Tooltip';
import {
  CpuChipIcon,
  CircleStackIcon,
  UsersIcon,
  PencilIcon,
  TrashIcon,
  SignalIcon,
} from '@heroicons/react/24/outline';

// ============================================================================
// Resource Group Card Component
// Displays a resource group with usage statistics and quick actions
// ============================================================================

export interface ResourceGroupCardProps {
  group: ResourceGroup;
  usage?: {
    cpuUsage: number;
    memoryUsage: number;
    activeConnections: number;
    activeQueries: number;
  };
  onEdit?: (group: ResourceGroup) => void;
  onDelete?: (group: ResourceGroup) => void;
  onClick?: (group: ResourceGroup) => void;
}

export function ResourceGroupCard({
  group,
  usage,
  onEdit,
  onDelete,
  onClick,
}: ResourceGroupCardProps) {
  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(2)} GB`;
  };

  const getUsageColor = (usage: number, limit: number) => {
    const percentage = (usage / limit) * 100;
    if (percentage >= 90) return 'bg-danger-500';
    if (percentage >= 75) return 'bg-warning-500';
    return 'bg-success-500';
  };

  const cpuPercentage = usage ? (usage.cpuUsage / group.cpuLimit) * 100 : 0;
  const memoryPercentage = usage
    ? (usage.memoryUsage / group.memoryLimit) * 100
    : 0;
  const connectionPercentage = usage
    ? (usage.activeConnections / group.maxConnections) * 100
    : 0;

  return (
    <Card
      hoverable={!!onClick}
      onClick={() => onClick?.(group)}
      className="relative"
    >
      <CardHeader
        title={
          <div className="flex items-center gap-3">
            <span className="text-lg font-semibold text-dark-100">
              {group.name}
            </span>
            <Badge variant={group.isEnabled ? 'success' : 'neutral'} size="sm">
              {group.isEnabled ? 'Enabled' : 'Disabled'}
            </Badge>
          </div>
        }
        subtitle={
          <div className="flex items-center gap-2 mt-1">
            <Tooltip content="Priority level">
              <div className="flex items-center gap-1 text-xs text-dark-400">
                <SignalIcon className="w-4 h-4" />
                Priority: {group.priority}
              </div>
            </Tooltip>
            <span className="text-dark-600">•</span>
            <Tooltip content="Number of members">
              <div className="flex items-center gap-1 text-xs text-dark-400">
                <UsersIcon className="w-4 h-4" />
                {group.members.length} members
              </div>
            </Tooltip>
          </div>
        }
        action={
          <div className="flex gap-2">
            {onEdit && (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onEdit(group);
                }}
                leftIcon={<PencilIcon className="w-4 h-4" />}
              >
                Edit
              </Button>
            )}
            {onDelete && (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(group);
                }}
                leftIcon={<TrashIcon className="w-4 h-4" />}
                className="text-danger-500 hover:text-danger-400"
              >
                Delete
              </Button>
            )}
          </div>
        }
      />

      <div className="space-y-4 mt-4">
        {/* CPU Usage */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2 text-sm text-dark-300">
              <CpuChipIcon className="w-4 h-4" />
              <span>CPU</span>
            </div>
            <div className="text-sm text-dark-400">
              {usage ? `${usage.cpuUsage.toFixed(1)}%` : '0%'} / {group.cpuLimit}%
            </div>
          </div>
          <div className="w-full bg-dark-700 rounded-full h-2 overflow-hidden">
            <div
              className={`h-full transition-all duration-300 ${
                usage ? getUsageColor(usage.cpuUsage, group.cpuLimit) : 'bg-dark-600'
              }`}
              style={{ width: `${Math.min(cpuPercentage, 100)}%` }}
            />
          </div>
        </div>

        {/* Memory Usage */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2 text-sm text-dark-300">
              <CircleStackIcon className="w-4 h-4" />
              <span>Memory</span>
            </div>
            <div className="text-sm text-dark-400">
              {usage ? formatBytes(usage.memoryUsage) : '0 GB'} /{' '}
              {formatBytes(group.memoryLimit)}
            </div>
          </div>
          <div className="w-full bg-dark-700 rounded-full h-2 overflow-hidden">
            <div
              className={`h-full transition-all duration-300 ${
                usage ? getUsageColor(usage.memoryUsage, group.memoryLimit) : 'bg-dark-600'
              }`}
              style={{ width: `${Math.min(memoryPercentage, 100)}%` }}
            />
          </div>
        </div>

        {/* Connections */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2 text-sm text-dark-300">
              <UsersIcon className="w-4 h-4" />
              <span>Connections</span>
            </div>
            <div className="text-sm text-dark-400">
              {usage?.activeConnections || 0} / {group.maxConnections}
            </div>
          </div>
          <div className="w-full bg-dark-700 rounded-full h-2 overflow-hidden">
            <div
              className={`h-full transition-all duration-300 ${
                usage
                  ? getUsageColor(usage.activeConnections, group.maxConnections)
                  : 'bg-dark-600'
              }`}
              style={{ width: `${Math.min(connectionPercentage, 100)}%` }}
            />
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 gap-4 pt-4 border-t border-dark-700">
          <div>
            <div className="text-xs text-dark-400 mb-1">Max Queries</div>
            <div className="text-sm font-semibold text-dark-200">
              {group.maxQueries}
            </div>
          </div>
          <div>
            <div className="text-xs text-dark-400 mb-1">Query Timeout</div>
            <div className="text-sm font-semibold text-dark-200">
              {group.queryTimeout}ms
            </div>
          </div>
          {usage && (
            <>
              <div>
                <div className="text-xs text-dark-400 mb-1">Active Queries</div>
                <div className="text-sm font-semibold text-dark-200">
                  {usage.activeQueries}
                </div>
              </div>
              <div>
                <div className="text-xs text-dark-400 mb-1">Queued</div>
                <div className="text-sm font-semibold text-dark-200">0</div>
              </div>
            </>
          )}
        </div>
      </div>
    </Card>
  );
}
