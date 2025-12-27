// ============================================================================
// RustyDB Health Indicator Component
// System health status with component breakdown and history timeline
// ============================================================================

import React, { useMemo } from 'react';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  ClockIcon,
} from '@heroicons/react/24/solid';
import {
  ServerIcon,
  CircleStackIcon,
  SignalIcon,
  CpuChipIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import { parseISO, formatDistance } from 'date-fns';
import type { HealthStatus, HealthState, ComponentHealth } from '../../types';
import { formatDuration } from '../../services/metricsService';

// ============================================================================
// Types
// ============================================================================

export interface HealthIndicatorProps {
  health: HealthStatus;
  showComponents?: boolean;
  showUptime?: boolean;
  showVersion?: boolean;
  isLoading?: boolean;
  error?: string | null;
  className?: string;
}

// ============================================================================
// Component Icons Map
// ============================================================================

const COMPONENT_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  storage: CircleStackIcon,
  compute: CpuChipIcon,
  network: SignalIcon,
  server: ServerIcon,
  database: CircleStackIcon,
  cache: ServerIcon,
  replication: SignalIcon,
};

// ============================================================================
// Main Component
// ============================================================================

export const HealthIndicator: React.FC<HealthIndicatorProps> = ({
  health,
  showComponents = true,
  showUptime = true,
  showVersion = true,
  isLoading = false,
  error = null,
  className,
}) => {
  // Get status badge info
  const getStatusInfo = (status: HealthState) => {
    switch (status) {
      case 'healthy':
        return {
          icon: CheckCircleIcon,
          label: 'Healthy',
          bgColor: 'bg-green-100',
          textColor: 'text-green-800',
          borderColor: 'border-green-200',
          dotColor: 'bg-green-500',
        };
      case 'degraded':
        return {
          icon: ExclamationTriangleIcon,
          label: 'Degraded',
          bgColor: 'bg-yellow-100',
          textColor: 'text-yellow-800',
          borderColor: 'border-yellow-200',
          dotColor: 'bg-yellow-500',
        };
      case 'unhealthy':
        return {
          icon: ExclamationTriangleIcon,
          label: 'Unhealthy',
          bgColor: 'bg-orange-100',
          textColor: 'text-orange-800',
          borderColor: 'border-orange-200',
          dotColor: 'bg-orange-500',
        };
      case 'critical':
        return {
          icon: XCircleIcon,
          label: 'Critical',
          bgColor: 'bg-red-100',
          textColor: 'text-red-800',
          borderColor: 'border-red-200',
          dotColor: 'bg-red-500',
        };
      default:
        return {
          icon: CheckCircleIcon,
          label: 'Unknown',
          bgColor: 'bg-gray-100',
          textColor: 'text-gray-800',
          borderColor: 'border-gray-200',
          dotColor: 'bg-gray-500',
        };
    }
  };

  const statusInfo = getStatusInfo(health.status);
  const StatusIcon = statusInfo.icon;

  // Sort components by status (critical first)
  const sortedComponents = useMemo(() => {
    if (!health.components) return [];

    const statusOrder: Record<HealthState, number> = {
      critical: 0,
      unhealthy: 1,
      degraded: 2,
      healthy: 3,
    };

    return [...health.components].sort(
      (a, b) => statusOrder[a.status] - statusOrder[b.status]
    );
  }, [health.components]);

  // Loading state
  if (isLoading) {
    return (
      <div
        className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
        role="status"
        aria-label="Loading health status"
      >
        <div className="animate-pulse space-y-4">
          <div className="flex items-center justify-between">
            <div className="h-8 w-48 bg-gray-300 rounded" />
            <div className="h-6 w-32 bg-gray-300 rounded-full" />
          </div>
          <div className="space-y-2">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 bg-gray-200 rounded" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div
        className={clsx('bg-white rounded-lg border border-red-200 p-6', className)}
        role="alert"
      >
        <div className="flex items-center space-x-3">
          <XCircleIcon className="h-6 w-6 text-red-600" />
          <div>
            <p className="text-sm font-medium text-red-600">
              Failed to load health status
            </p>
            <p className="text-xs text-red-500 mt-1">{error}</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={clsx('bg-white rounded-lg border border-gray-200', className)}>
      {/* Header */}
      <div className="p-6 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <StatusIcon className={clsx('h-8 w-8', statusInfo.textColor)} />
            <div>
              <h3 className="text-lg font-semibold text-gray-900">System Health</h3>
              <p className="text-sm text-gray-500">
                Last checked{' '}
                {formatDistance(parseISO(health.timestamp), new Date(), {
                  addSuffix: true,
                })}
              </p>
            </div>
          </div>

          {/* Status Badge */}
          <div
            className={clsx(
              'flex items-center space-x-2 px-4 py-2 rounded-full border',
              statusInfo.bgColor,
              statusInfo.textColor,
              statusInfo.borderColor
            )}
          >
            <span className={clsx('h-2 w-2 rounded-full', statusInfo.dotColor)} />
            <span className="text-sm font-semibold">{statusInfo.label}</span>
          </div>
        </div>

        {/* Meta info */}
        <div className="mt-4 flex items-center space-x-6 text-sm text-gray-600">
          {showUptime && (
            <div className="flex items-center space-x-2">
              <ClockIcon className="h-4 w-4" />
              <span>Uptime: {formatDuration(health.uptime)}</span>
            </div>
          )}
          {showVersion && (
            <div className="flex items-center space-x-2">
              <ServerIcon className="h-4 w-4" />
              <span>Version: {health.version}</span>
            </div>
          )}
        </div>
      </div>

      {/* Component Health */}
      {showComponents && sortedComponents.length > 0 && (
        <div className="p-6">
          <h4 className="text-sm font-semibold text-gray-900 mb-4">
            Component Health
          </h4>
          <div className="space-y-3">
            {sortedComponents.map((component) => (
              <ComponentHealthItem key={component.name} component={component} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

// ============================================================================
// Component Health Item
// ============================================================================

interface ComponentHealthItemProps {
  component: ComponentHealth;
}

const ComponentHealthItem: React.FC<ComponentHealthItemProps> = ({ component }) => {
  const statusInfo = useMemo(() => {
    switch (component.status) {
      case 'healthy':
        return {
          color: 'text-green-600',
          bgColor: 'bg-green-50',
          borderColor: 'border-green-200',
          icon: CheckCircleIcon,
        };
      case 'degraded':
        return {
          color: 'text-yellow-600',
          bgColor: 'bg-yellow-50',
          borderColor: 'border-yellow-200',
          icon: ExclamationTriangleIcon,
        };
      case 'unhealthy':
        return {
          color: 'text-orange-600',
          bgColor: 'bg-orange-50',
          borderColor: 'border-orange-200',
          icon: ExclamationTriangleIcon,
        };
      case 'critical':
        return {
          color: 'text-red-600',
          bgColor: 'bg-red-50',
          borderColor: 'border-red-200',
          icon: XCircleIcon,
        };
      default:
        return {
          color: 'text-gray-600',
          bgColor: 'bg-gray-50',
          borderColor: 'border-gray-200',
          icon: CheckCircleIcon,
        };
    }
  }, [component.status]);

  const ComponentIcon =
    COMPONENT_ICONS[component.name.toLowerCase()] || ServerIcon;
  const StatusIcon = statusInfo.icon;

  return (
    <div
      className={clsx(
        'flex items-center justify-between p-4 rounded-lg border',
        statusInfo.bgColor,
        statusInfo.borderColor
      )}
    >
      <div className="flex items-center space-x-3 flex-1 min-w-0">
        <div className={clsx('rounded-lg p-2', statusInfo.bgColor)}>
          <ComponentIcon className={clsx('h-5 w-5', statusInfo.color)} />
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center space-x-2">
            <h5 className="text-sm font-medium text-gray-900 truncate">
              {component.name}
            </h5>
            <StatusIcon className={clsx('h-4 w-4 shrink-0', statusInfo.color)} />
          </div>

          {component.message && (
            <p className="text-xs text-gray-600 mt-1 truncate" title={component.message}>
              {component.message}
            </p>
          )}

          <div className="flex items-center space-x-4 mt-1 text-xs text-gray-500">
            <span>
              Checked{' '}
              {formatDistance(parseISO(component.lastCheck), new Date(), {
                addSuffix: true,
              })}
            </span>
            {component.responseTime !== undefined && (
              <span>{component.responseTime}ms</span>
            )}
          </div>
        </div>
      </div>

      <div className="ml-4">
        <span
          className={clsx(
            'inline-block px-2 py-1 text-xs font-medium rounded capitalize',
            statusInfo.color,
            statusInfo.bgColor
          )}
        >
          {component.status}
        </span>
      </div>
    </div>
  );
};

// ============================================================================
// Compact Health Badge Component
// ============================================================================

export interface CompactHealthBadgeProps {
  status: HealthState;
  label?: string;
  size?: 'sm' | 'md' | 'lg';
  showIcon?: boolean;
  className?: string;
}

export const CompactHealthBadge: React.FC<CompactHealthBadgeProps> = ({
  status,
  label,
  size = 'md',
  showIcon = true,
  className,
}) => {
  const statusInfo = useMemo(() => {
    switch (status) {
      case 'healthy':
        return {
          icon: CheckCircleIcon,
          label: label || 'Healthy',
          bgColor: 'bg-green-100',
          textColor: 'text-green-800',
          borderColor: 'border-green-200',
        };
      case 'degraded':
        return {
          icon: ExclamationTriangleIcon,
          label: label || 'Degraded',
          bgColor: 'bg-yellow-100',
          textColor: 'text-yellow-800',
          borderColor: 'border-yellow-200',
        };
      case 'unhealthy':
        return {
          icon: ExclamationTriangleIcon,
          label: label || 'Unhealthy',
          bgColor: 'bg-orange-100',
          textColor: 'text-orange-800',
          borderColor: 'border-orange-200',
        };
      case 'critical':
        return {
          icon: XCircleIcon,
          label: label || 'Critical',
          bgColor: 'bg-red-100',
          textColor: 'text-red-800',
          borderColor: 'border-red-200',
        };
      default:
        return {
          icon: CheckCircleIcon,
          label: label || 'Unknown',
          bgColor: 'bg-gray-100',
          textColor: 'text-gray-800',
          borderColor: 'border-gray-200',
        };
    }
  }, [status, label]);

  const Icon = statusInfo.icon;

  const sizeClasses = {
    sm: 'px-2 py-1 text-xs',
    md: 'px-3 py-1.5 text-sm',
    lg: 'px-4 py-2 text-base',
  };

  const iconSizeClasses = {
    sm: 'h-3 w-3',
    md: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  return (
    <span
      className={clsx(
        'inline-flex items-center space-x-1.5 font-medium rounded-full border',
        statusInfo.bgColor,
        statusInfo.textColor,
        statusInfo.borderColor,
        sizeClasses[size],
        className
      )}
    >
      {showIcon && <Icon className={iconSizeClasses[size]} />}
      <span>{statusInfo.label}</span>
    </span>
  );
};

// ============================================================================
// Skeleton Loader
// ============================================================================

export const HealthIndicatorSkeleton: React.FC<{ className?: string }> = ({
  className,
}) => {
  return (
    <div
      className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
    >
      <div className="animate-pulse space-y-4">
        <div className="flex items-center justify-between">
          <div className="h-8 w-48 bg-gray-300 rounded" />
          <div className="h-6 w-32 bg-gray-300 rounded-full" />
        </div>
        <div className="space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-16 bg-gray-200 rounded" />
          ))}
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// Export
// ============================================================================

export default HealthIndicator;
