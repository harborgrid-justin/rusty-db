// ============================================================================
// RustyDB Performance Chart Component
// Time series chart with Recharts, multiple series, time range selector
// ============================================================================

import React, { useMemo } from 'react';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  TooltipProps,
} from 'recharts';
import { format, parseISO } from 'date-fns';
import clsx from 'clsx';
import { TimeRange, PerformanceDataPoint } from '../../services/metricsService';

// ============================================================================
// Types
// ============================================================================

export interface ChartSeries {
  key: string;
  name: string;
  data: PerformanceDataPoint[];
  color: string;
  unit?: string;
}

export interface PerformanceChartProps {
  title: string;
  series: ChartSeries[];
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
  chartType?: 'line' | 'area';
  height?: number;
  isLoading?: boolean;
  error?: string | null;
  className?: string;
  showLegend?: boolean;
  showGrid?: boolean;
  curve?: 'monotone' | 'linear' | 'step';
}

// ============================================================================
// Time Range Options
// ============================================================================

const TIME_RANGE_OPTIONS: Array<{ value: TimeRange; label: string }> = [
  { value: '1h', label: '1 Hour' },
  { value: '6h', label: '6 Hours' },
  { value: '24h', label: '24 Hours' },
  { value: '7d', label: '7 Days' },
  { value: '30d', label: '30 Days' },
];

// ============================================================================
// Component
// ============================================================================

export const PerformanceChart: React.FC<PerformanceChartProps> = ({
  title,
  series,
  timeRange,
  onTimeRangeChange,
  chartType = 'line',
  height = 300,
  isLoading = false,
  error = null,
  className,
  showLegend = true,
  showGrid = true,
  curve = 'monotone',
}) => {
  // Prepare chart data by merging all series
  const chartData = useMemo(() => {
    if (!series || series.length === 0) return [];

    // Get all unique timestamps
    const timestampSet = new Set<string>();
    series.forEach((s) => {
      s.data.forEach((point) => {
        timestampSet.add(point.timestamp);
      });
    });

    const timestamps = Array.from(timestampSet).sort();

    // Create data points for each timestamp
    return timestamps.map((timestamp) => {
      const point: Record<string, unknown> = { timestamp };

      series.forEach((s) => {
        const dataPoint = s.data.find((d) => d.timestamp === timestamp);
        point[s.key] = dataPoint?.value ?? null;
      });

      return point;
    });
  }, [series]);

  // Format timestamp based on time range
  const formatTimestamp = (timestamp: string): string => {
    try {
      const date = parseISO(timestamp);

      switch (timeRange) {
        case '1h':
        case '6h':
          return format(date, 'HH:mm');
        case '24h':
          return format(date, 'HH:mm');
        case '7d':
          return format(date, 'MMM dd');
        case '30d':
          return format(date, 'MMM dd');
        default:
          return format(date, 'HH:mm');
      }
    } catch {
      return timestamp;
    }
  };

  // Format value with unit
  const formatValue = (value: number, unit?: string): string => {
    if (unit === '%') {
      return `${value.toFixed(1)}%`;
    }
    if (unit === 'ms') {
      return `${value.toFixed(0)}ms`;
    }
    if (unit === 'MB') {
      return `${value.toFixed(1)}MB`;
    }
    return value.toLocaleString();
  };

  // Custom tooltip
  const CustomTooltip: React.FC<TooltipProps<number, string>> = ({
    active,
    payload,
    label,
  }) => {
    if (!active || !payload || payload.length === 0) {
      return null;
    }

    return (
      <div className="bg-white rounded-lg shadow-lg border border-gray-200 p-3">
        <p className="text-xs font-medium text-gray-600 mb-2">
          {formatTimestamp(label)}
        </p>
        <div className="space-y-1">
          {payload.map((entry) => {
            const seriesInfo = series.find((s) => s.key === entry.dataKey);
            return (
              <div key={entry.dataKey} className="flex items-center justify-between gap-4">
                <div className="flex items-center gap-2">
                  <div
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: entry.color }}
                  />
                  <span className="text-xs text-gray-700">{entry.name}</span>
                </div>
                <span className="text-xs font-semibold text-gray-900">
                  {formatValue(entry.value as number, seriesInfo?.unit)}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  // Loading state
  if (isLoading) {
    return (
      <div
        className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
        role="status"
        aria-label="Loading chart"
      >
        <div className="flex items-center justify-between mb-4">
          <div className="h-6 w-48 bg-gray-300 rounded animate-pulse" />
          <div className="h-8 w-64 bg-gray-300 rounded animate-pulse" />
        </div>
        <div
          className="bg-gray-200 rounded animate-pulse"
          style={{ height: `${height}px` }}
        />
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
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
          <TimeRangeSelector
            value={timeRange}
            onChange={onTimeRangeChange}
            disabled
          />
        </div>
        <div
          className="flex items-center justify-center bg-red-50 rounded"
          style={{ height: `${height}px` }}
        >
          <div className="text-center">
            <p className="text-sm font-medium text-red-600">Failed to load chart</p>
            <p className="text-xs text-red-500 mt-1">{error}</p>
          </div>
        </div>
      </div>
    );
  }

  // Empty state
  if (!chartData || chartData.length === 0) {
    return (
      <div
        className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
      >
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
          <TimeRangeSelector value={timeRange} onChange={onTimeRangeChange} />
        </div>
        <div
          className="flex items-center justify-center bg-gray-50 rounded"
          style={{ height: `${height}px` }}
        >
          <p className="text-sm text-gray-500">No data available</p>
        </div>
      </div>
    );
  }

  return (
    <div
      className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        <TimeRangeSelector value={timeRange} onChange={onTimeRangeChange} />
      </div>

      {/* Chart */}
      <ResponsiveContainer width="100%" height={height}>
        {chartType === 'area' ? (
          <AreaChart data={chartData} margin={{ top: 5, right: 5, left: 0, bottom: 5 }}>
            {showGrid && (
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" vertical={false} />
            )}
            <XAxis
              dataKey="timestamp"
              tickFormatter={formatTimestamp}
              stroke="#6b7280"
              style={{ fontSize: '12px' }}
              tickLine={false}
            />
            <YAxis
              stroke="#6b7280"
              style={{ fontSize: '12px' }}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip content={<CustomTooltip />} />
            {showLegend && (
              <Legend
                wrapperStyle={{ fontSize: '12px' }}
                iconType="circle"
                iconSize={8}
              />
            )}
            {series.map((s) => (
              <Area
                key={s.key}
                type={curve}
                dataKey={s.key}
                name={s.name}
                stroke={s.color}
                fill={s.color}
                fillOpacity={0.2}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4 }}
              />
            ))}
          </AreaChart>
        ) : (
          <LineChart data={chartData} margin={{ top: 5, right: 5, left: 0, bottom: 5 }}>
            {showGrid && (
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" vertical={false} />
            )}
            <XAxis
              dataKey="timestamp"
              tickFormatter={formatTimestamp}
              stroke="#6b7280"
              style={{ fontSize: '12px' }}
              tickLine={false}
            />
            <YAxis
              stroke="#6b7280"
              style={{ fontSize: '12px' }}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip content={<CustomTooltip />} />
            {showLegend && (
              <Legend
                wrapperStyle={{ fontSize: '12px' }}
                iconType="circle"
                iconSize={8}
              />
            )}
            {series.map((s) => (
              <Line
                key={s.key}
                type={curve}
                dataKey={s.key}
                name={s.name}
                stroke={s.color}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4 }}
              />
            ))}
          </LineChart>
        )}
      </ResponsiveContainer>
    </div>
  );
};

// ============================================================================
// Time Range Selector Component
// ============================================================================

interface TimeRangeSelectorProps {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  disabled?: boolean;
}

const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  value,
  onChange,
  disabled = false,
}) => {
  return (
    <div
      className="inline-flex rounded-lg border border-gray-300 bg-white p-1"
      role="group"
      aria-label="Time range selector"
    >
      {TIME_RANGE_OPTIONS.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => onChange(option.value)}
          disabled={disabled}
          className={clsx(
            'px-3 py-1 text-sm font-medium rounded-md transition-colors',
            'focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1',
            value === option.value
              ? 'bg-blue-600 text-white'
              : 'text-gray-700 hover:bg-gray-100',
            disabled && 'opacity-50 cursor-not-allowed'
          )}
          aria-pressed={value === option.value}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
};

// ============================================================================
// Chart Skeleton Loader
// ============================================================================

export const PerformanceChartSkeleton: React.FC<{
  height?: number;
  className?: string;
}> = ({ height = 300, className }) => {
  return (
    <div
      className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
    >
      <div className="flex items-center justify-between mb-4">
        <div className="h-6 w-48 bg-gray-300 rounded animate-pulse" />
        <div className="h-8 w-64 bg-gray-300 rounded animate-pulse" />
      </div>
      <div
        className="bg-gray-200 rounded animate-pulse"
        style={{ height: `${height}px` }}
      />
    </div>
  );
};

// ============================================================================
// Export
// ============================================================================

export default PerformanceChart;
