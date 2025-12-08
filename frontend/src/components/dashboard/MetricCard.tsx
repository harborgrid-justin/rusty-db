// ============================================================================
// RustyDB Metric Card Component
// Reusable metric display card with sparkline, change indicators, and states
// ============================================================================

import React from 'react';
import { ArrowUpIcon, ArrowDownIcon, MinusIcon } from '@heroicons/react/24/solid';
import clsx from 'clsx';

// ============================================================================
// Types
// ============================================================================

export interface MetricCardProps {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string | number;
  unit?: string;
  change?: number;
  changeLabel?: string;
  sparklineData?: number[];
  trend?: 'up' | 'down' | 'stable';
  status?: 'normal' | 'warning' | 'critical' | 'success';
  isLoading?: boolean;
  error?: string | null;
  onClick?: () => void;
  className?: string;
}

// ============================================================================
// Component
// ============================================================================

export const MetricCard: React.FC<MetricCardProps> = ({
  icon: Icon,
  label,
  value,
  unit,
  change,
  changeLabel = 'vs last period',
  sparklineData,
  trend,
  status = 'normal',
  isLoading = false,
  error = null,
  onClick,
  className,
}) => {
  // Format change percentage
  const formatChange = (val: number): string => {
    const sign = val >= 0 ? '+' : '';
    return `${sign}${val.toFixed(1)}%`;
  };

  // Get status colors
  const getStatusColor = () => {
    switch (status) {
      case 'success':
        return 'border-green-200 bg-green-50';
      case 'warning':
        return 'border-yellow-200 bg-yellow-50';
      case 'critical':
        return 'border-red-200 bg-red-50';
      default:
        return 'border-gray-200 bg-white';
    }
  };

  // Get icon color
  const getIconColor = () => {
    switch (status) {
      case 'success':
        return 'text-green-600 bg-green-100';
      case 'warning':
        return 'text-yellow-600 bg-yellow-100';
      case 'critical':
        return 'text-red-600 bg-red-100';
      default:
        return 'text-blue-600 bg-blue-100';
    }
  };

  // Get trend icon and color
  const getTrendIcon = () => {
    if (trend === 'up' || (change !== undefined && change > 0)) {
      return <ArrowUpIcon className="h-4 w-4" />;
    }
    if (trend === 'down' || (change !== undefined && change < 0)) {
      return <ArrowDownIcon className="h-4 w-4" />;
    }
    return <MinusIcon className="h-4 w-4" />;
  };

  const getTrendColor = () => {
    if (trend === 'up' || (change !== undefined && change > 0)) {
      return 'text-green-600 bg-green-100';
    }
    if (trend === 'down' || (change !== undefined && change < 0)) {
      return 'text-red-600 bg-red-100';
    }
    return 'text-gray-600 bg-gray-100';
  };

  // Loading state
  if (isLoading) {
    return (
      <div
        className={clsx(
          'rounded-lg border-2 p-6 animate-pulse',
          getStatusColor(),
          onClick && 'cursor-pointer hover:shadow-md transition-shadow',
          className
        )}
        role="status"
        aria-label="Loading metric"
      >
        <div className="flex items-start justify-between">
          <div className="flex-1 space-y-3">
            <div className="h-4 bg-gray-300 rounded w-24" />
            <div className="h-8 bg-gray-300 rounded w-32" />
            <div className="h-3 bg-gray-300 rounded w-20" />
          </div>
          <div className="h-12 w-12 bg-gray-300 rounded-lg" />
        </div>
        {sparklineData && (
          <div className="mt-4 h-16 bg-gray-200 rounded" />
        )}
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div
        className={clsx(
          'rounded-lg border-2 border-red-200 bg-red-50 p-6',
          onClick && 'cursor-pointer hover:shadow-md transition-shadow',
          className
        )}
        role="alert"
        aria-label="Error loading metric"
      >
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <p className="text-sm font-medium text-gray-700">{label}</p>
            <p className="mt-2 text-sm text-red-600">Failed to load</p>
            <p className="mt-1 text-xs text-red-500">{error}</p>
          </div>
          <div className={clsx('rounded-lg p-3', 'text-red-600 bg-red-100')}>
            <Icon className="h-6 w-6" aria-hidden="true" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={clsx(
        'rounded-lg border-2 p-6 transition-all duration-200',
        getStatusColor(),
        onClick && 'cursor-pointer hover:shadow-md hover:scale-[1.02]',
        className
      )}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={
        onClick
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onClick();
              }
            }
          : undefined
      }
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          {/* Label */}
          <p className="text-sm font-medium text-gray-700 truncate" title={label}>
            {label}
          </p>

          {/* Value */}
          <div className="mt-2 flex items-baseline">
            <p className="text-3xl font-bold text-gray-900 tracking-tight">
              {typeof value === 'number' ? value.toLocaleString() : value}
            </p>
            {unit && (
              <span className="ml-2 text-sm font-medium text-gray-500">{unit}</span>
            )}
          </div>

          {/* Change indicator */}
          {change !== undefined && (
            <div className="mt-2 flex items-center space-x-1">
              <span
                className={clsx(
                  'inline-flex items-center space-x-1 rounded-full px-2 py-0.5 text-xs font-medium',
                  getTrendColor()
                )}
              >
                {getTrendIcon()}
                <span>{formatChange(change)}</span>
              </span>
              <span className="text-xs text-gray-500">{changeLabel}</span>
            </div>
          )}
        </div>

        {/* Icon */}
        <div
          className={clsx('rounded-lg p-3 shrink-0', getIconColor())}
          aria-hidden="true"
        >
          <Icon className="h-6 w-6" />
        </div>
      </div>

      {/* Sparkline */}
      {sparklineData && sparklineData.length > 0 && (
        <div className="mt-4" aria-label="Sparkline chart">
          <Sparkline data={sparklineData} color={status} />
        </div>
      )}
    </div>
  );
};

// ============================================================================
// Sparkline Component
// ============================================================================

interface SparklineProps {
  data: number[];
  color?: 'normal' | 'warning' | 'critical' | 'success';
  height?: number;
}

const Sparkline: React.FC<SparklineProps> = ({
  data,
  color = 'normal',
  height = 40,
}) => {
  if (data.length < 2) {
    return null;
  }

  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;

  const width = 200;
  const padding = 2;
  const stepX = (width - padding * 2) / (data.length - 1);

  // Generate SVG path
  const points = data.map((value, index) => {
    const x = padding + index * stepX;
    const y = height - padding - ((value - min) / range) * (height - padding * 2);
    return `${x},${y}`;
  });

  const pathData = `M ${points.join(' L ')}`;

  // Get color based on status
  const getColor = () => {
    switch (color) {
      case 'success':
        return '#16a34a'; // green-600
      case 'warning':
        return '#ca8a04'; // yellow-600
      case 'critical':
        return '#dc2626'; // red-600
      default:
        return '#2563eb'; // blue-600
    }
  };

  return (
    <svg
      width="100%"
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      preserveAspectRatio="none"
      className="overflow-visible"
      aria-hidden="true"
    >
      {/* Area fill */}
      <defs>
        <linearGradient id={`gradient-${color}`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor={getColor()} stopOpacity="0.3" />
          <stop offset="100%" stopColor={getColor()} stopOpacity="0.05" />
        </linearGradient>
      </defs>
      <path
        d={`${pathData} L ${width - padding},${height} L ${padding},${height} Z`}
        fill={`url(#gradient-${color})`}
        opacity="0.5"
      />

      {/* Line */}
      <path
        d={pathData}
        fill="none"
        stroke={getColor()}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
};

// ============================================================================
// Skeleton Loader
// ============================================================================

export const MetricCardSkeleton: React.FC<{ className?: string }> = ({ className }) => {
  return (
    <div
      className={clsx(
        'rounded-lg border-2 border-gray-200 bg-white p-6 animate-pulse',
        className
      )}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 space-y-3">
          <div className="h-4 bg-gray-300 rounded w-24" />
          <div className="h-8 bg-gray-300 rounded w-32" />
          <div className="h-3 bg-gray-300 rounded w-20" />
        </div>
        <div className="h-12 w-12 bg-gray-300 rounded-lg" />
      </div>
      <div className="mt-4 h-10 bg-gray-200 rounded" />
    </div>
  );
};

// ============================================================================
// Export
// ============================================================================

export default MetricCard;
