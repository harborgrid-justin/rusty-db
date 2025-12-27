// ============================================================================
// Replication Lag Chart Component
// Time series visualization of replication lag
// ============================================================================

import { useState, useMemo } from 'react';
import { format } from 'date-fns';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts';
import { ClockIcon } from '@heroicons/react/24/outline';
import type { ReplicationLagMetric } from '../../services/clusterService';

interface ReplicationLagChartProps {
  data: ReplicationLagMetric[];
  nodeName?: string;
  thresholds?: {
    warning: number; // milliseconds
    critical: number; // milliseconds
  };
  className?: string;
}

const DEFAULT_THRESHOLDS = {
  warning: 1000, // 1 second
  critical: 5000, // 5 seconds
};

export function ReplicationLagChart({
  data,
  nodeName,
  thresholds = DEFAULT_THRESHOLDS,
  className = '',
}: ReplicationLagChartProps) {
  const [timeRange, setTimeRange] = useState<'1h' | '6h' | '24h' | 'all'>('1h');

  // Filter and format data based on time range
  const chartData = useMemo(() => {
    const now = new Date();
    let cutoffTime: Date;

    switch (timeRange) {
      case '1h':
        cutoffTime = new Date(now.getTime() - 60 * 60 * 1000);
        break;
      case '6h':
        cutoffTime = new Date(now.getTime() - 6 * 60 * 60 * 1000);
        break;
      case '24h':
        cutoffTime = new Date(now.getTime() - 24 * 60 * 60 * 1000);
        break;
      default:
        cutoffTime = new Date(0);
    }

    return data
      .filter((point) => new Date(point.timestamp) >= cutoffTime)
      .map((point) => ({
        timestamp: new Date(point.timestamp).getTime(),
        lag: point.lag / 1000, // Convert to seconds
        bytesPerSecond: point.bytesPerSecond,
        transactionsPerSecond: point.transactionsPerSecond,
      }))
      .sort((a, b) => a.timestamp - b.timestamp);
  }, [data, timeRange]);

  // Calculate statistics
  const stats = useMemo(() => {
    if (chartData.length === 0) {
      return {
        current: 0,
        avg: 0,
        min: 0,
        max: 0,
        p95: 0,
      };
    }

    const lags = chartData.map((d) => d.lag);
    const sortedLags = [...lags].sort((a, b) => a - b);

    return {
      current: lags[lags.length - 1] || 0,
      avg: lags.reduce((sum, lag) => sum + lag, 0) / lags.length,
      min: Math.min(...lags),
      max: Math.max(...lags),
      p95: sortedLags[Math.floor(sortedLags.length * 0.95)] || 0,
    };
  }, [chartData]);

  function formatXAxis(timestamp: number) {
    const date = new Date(timestamp);

    switch (timeRange) {
      case '1h':
        return format(date, 'HH:mm');
      case '6h':
        return format(date, 'HH:mm');
      case '24h':
        return format(date, 'HH:mm');
      default:
        return format(date, 'MM/dd HH:mm');
    }
  }

  interface TooltipPayload {
    payload: {
      timestamp: number;
      lag: number;
      bytesPerSecond: number;
      transactionsPerSecond: number;
    };
  }

  interface CustomTooltipProps {
    active?: boolean;
    payload?: TooltipPayload[];
  }

  function CustomTooltip({ active, payload }: CustomTooltipProps) {
    if (!active || !payload || !payload.length) {
      return null;
    }

    const data = payload[0].payload;

    return (
      <div className="bg-white border border-gray-200 rounded-lg shadow-lg p-3">
        <div className="text-xs text-gray-500 mb-2">
          {format(new Date(data.timestamp), 'PPpp')}
        </div>
        <div className="space-y-1">
          <div className="flex items-center justify-between space-x-4">
            <span className="text-sm text-gray-600">Lag:</span>
            <span className="text-sm font-semibold text-gray-900">
              {data.lag.toFixed(3)}s
            </span>
          </div>
          <div className="flex items-center justify-between space-x-4">
            <span className="text-sm text-gray-600">Bytes/s:</span>
            <span className="text-sm font-semibold text-gray-900">
              {formatBytes(data.bytesPerSecond)}
            </span>
          </div>
          <div className="flex items-center justify-between space-x-4">
            <span className="text-sm text-gray-600">Txn/s:</span>
            <span className="text-sm font-semibold text-gray-900">
              {data.transactionsPerSecond.toFixed(0)}
            </span>
          </div>
        </div>
      </div>
    );
  }

  function formatBytes(bytes: number): string {
    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  }

  return (
    <div className={`bg-white border border-gray-200 rounded-lg shadow-sm ${className}`}>
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <ClockIcon className="w-5 h-5 text-gray-600" />
            <h3 className="text-lg font-semibold text-gray-900">
              Replication Lag History
            </h3>
            {nodeName && (
              <span className="text-sm text-gray-500">- {nodeName}</span>
            )}
          </div>

          {/* Time Range Selector */}
          <div className="flex space-x-1 bg-gray-100 rounded-lg p-1">
            {(['1h', '6h', '24h', 'all'] as const).map((range) => (
              <button
                key={range}
                onClick={() => setTimeRange(range)}
                className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
                  timeRange === range
                    ? 'bg-white text-gray-900 shadow-sm'
                    : 'text-gray-600 hover:text-gray-900'
                }`}
              >
                {range.toUpperCase()}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Statistics */}
      <div className="grid grid-cols-5 gap-4 px-4 py-3 border-b border-gray-200 bg-gray-50">
        <div>
          <div className="text-xs text-gray-500">Current</div>
          <div className="text-lg font-semibold text-gray-900">
            {stats.current.toFixed(3)}s
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Average</div>
          <div className="text-lg font-semibold text-gray-900">
            {stats.avg.toFixed(3)}s
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Min</div>
          <div className="text-lg font-semibold text-green-600">
            {stats.min.toFixed(3)}s
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Max</div>
          <div className="text-lg font-semibold text-red-600">
            {stats.max.toFixed(3)}s
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">P95</div>
          <div className="text-lg font-semibold text-gray-900">
            {stats.p95.toFixed(3)}s
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="p-4">
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                dataKey="timestamp"
                tickFormatter={formatXAxis}
                stroke="#6b7280"
                style={{ fontSize: '12px' }}
              />
              <YAxis
                label={{
                  value: 'Lag (seconds)',
                  angle: -90,
                  position: 'insideLeft',
                  style: { fontSize: '12px', fill: '#6b7280' },
                }}
                stroke="#6b7280"
                style={{ fontSize: '12px' }}
              />
              <Tooltip content={<CustomTooltip />} />
              <Legend
                wrapperStyle={{ fontSize: '12px' }}
                iconType="line"
              />

              {/* Threshold lines */}
              <ReferenceLine
                y={thresholds.warning / 1000}
                stroke="#f59e0b"
                strokeDasharray="5 5"
                label={{
                  value: 'Warning',
                  position: 'right',
                  fill: '#f59e0b',
                  fontSize: 11,
                }}
              />
              <ReferenceLine
                y={thresholds.critical / 1000}
                stroke="#ef4444"
                strokeDasharray="5 5"
                label={{
                  value: 'Critical',
                  position: 'right',
                  fill: '#ef4444',
                  fontSize: 11,
                }}
              />

              {/* Main line */}
              <Line
                type="monotone"
                dataKey="lag"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={false}
                name="Replication Lag"
                isAnimationActive={false}
              />
            </LineChart>
          </ResponsiveContainer>
        ) : (
          <div className="h-[300px] flex items-center justify-center text-gray-500">
            <div className="text-center">
              <ClockIcon className="w-12 h-12 mx-auto text-gray-400 mb-2" />
              <p>No lag data available</p>
              <p className="text-sm mt-1">
                Data will appear once replication is active
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Legend */}
      <div className="px-4 py-3 border-t border-gray-200 bg-gray-50">
        <div className="flex items-center space-x-6 text-xs">
          <div className="flex items-center space-x-2">
            <div className="w-3 h-0.5 bg-amber-500"></div>
            <span className="text-gray-600">
              Warning Threshold ({(thresholds.warning / 1000).toFixed(1)}s)
            </span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-3 h-0.5 bg-red-500"></div>
            <span className="text-gray-600">
              Critical Threshold ({(thresholds.critical / 1000).toFixed(1)}s)
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
