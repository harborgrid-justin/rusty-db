// ============================================================================
// RustyDB Dashboard Page
// Main dashboard with system overview, metrics, charts, and activity feed
// ============================================================================

import React, { useState } from 'react';
import {
  CpuChipIcon,
  CircleStackIcon,
  SignalIcon,
  ChartBarIcon,
  ClockIcon,
  ServerIcon,
  ArrowPathIcon,
  Cog6ToothIcon,
  TableCellsIcon,
  BoltIcon,
} from '@heroicons/react/24/outline';
import { ExclamationTriangleIcon, BellAlertIcon } from '@heroicons/react/24/solid';
import clsx from 'clsx';
import MetricCard from '../components/dashboard/MetricCard';
import PerformanceChart, { ChartSeries } from '../components/dashboard/PerformanceChart';
import HealthIndicator, { CompactHealthBadge } from '../components/dashboard/HealthIndicator';
import ActivityFeed from '../components/dashboard/ActivityFeed';
import {
  useDashboardMetrics,
  usePerformanceChart,
  useActivityEvents,
  useAlerts,
  useMetricsCleanup,
} from '../hooks/useMetrics';
import { TimeRange, formatBytes, formatPercentage } from '../services/metricsService';

// ============================================================================
// Dashboard Page Component
// ============================================================================

export const Dashboard: React.FC = () => {
  const [timeRange, setTimeRange] = useState<TimeRange>('6h');
  const [realtimeEnabled, setRealtimeEnabled] = useState(true);

  // Cleanup WebSocket on unmount
  useMetricsCleanup();

  // Fetch dashboard data
  const {
    system,
    database,
    queries,
    health,
    alerts,
    isLoading,
    isError,
    error,
  } = useDashboardMetrics({ enabled: true, realtime: realtimeEnabled });

  // Fetch performance chart data
  const performanceQuery = usePerformanceChart({ timeRange });

  // Fetch activity events
  const activityQuery = useActivityEvents({
    limit: 50,
    realtime: realtimeEnabled,
  });

  // Fetch unacknowledged alerts
  const alertsQuery = useAlerts({
    acknowledged: false,
    realtime: realtimeEnabled,
  });

  // Prepare chart series
  const performanceChartSeries: ChartSeries[] = React.useMemo(() => {
    if (!performanceQuery.data) return [];

    return [
      {
        key: 'queriesPerSecond',
        name: 'Queries/sec',
        data: performanceQuery.data.metrics.queriesPerSecond,
        color: '#3b82f6', // blue-600
        unit: 'q/s',
      },
      {
        key: 'avgResponseTime',
        name: 'Avg Response Time',
        data: performanceQuery.data.metrics.avgResponseTime,
        color: '#10b981', // green-600
        unit: 'ms',
      },
    ];
  }, [performanceQuery.data]);

  const resourceChartSeries: ChartSeries[] = React.useMemo(() => {
    if (!performanceQuery.data) return [];

    return [
      {
        key: 'cpuUsage',
        name: 'CPU Usage',
        data: performanceQuery.data.metrics.cpuUsage,
        color: '#f59e0b', // amber-600
        unit: '%',
      },
      {
        key: 'memoryUsage',
        name: 'Memory Usage',
        data: performanceQuery.data.metrics.memoryUsage,
        color: '#8b5cf6', // violet-600
        unit: '%',
      },
    ];
  }, [performanceQuery.data]);

  // Calculate sparkline data for metric cards
  const getSparklineData = (dataPoints: { value: number }[] | undefined) => {
    if (!dataPoints || dataPoints.length === 0) return undefined;
    return dataPoints.map((point) => point.value).slice(-20);
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
              <p className="mt-1 text-sm text-gray-500">
                Monitor your database performance and system health
              </p>
            </div>

            <div className="flex items-center space-x-4">
              {/* Real-time toggle */}
              <label className="flex items-center space-x-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={realtimeEnabled}
                  onChange={(e) => setRealtimeEnabled(e.target.checked)}
                  className="h-4 w-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm font-medium text-gray-700">Real-time</span>
                {realtimeEnabled && (
                  <span className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                )}
              </label>

              {/* Health badge */}
              {health && <CompactHealthBadge status={health.status} size="md" />}

              {/* Quick actions */}
              <div className="flex items-center space-x-2">
                <button
                  type="button"
                  className="p-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
                  title="Settings"
                >
                  <Cog6ToothIcon className="h-5 w-5" />
                </button>
                <button
                  type="button"
                  className="relative p-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
                  title="Alerts"
                >
                  <BellAlertIcon className="h-5 w-5" />
                  {alertsQuery.data && alertsQuery.data.length > 0 && (
                    <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-red-500" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Error state */}
        {isError && (
          <div
            className="mb-6 bg-red-50 border border-red-200 rounded-lg p-4"
            role="alert"
          >
            <div className="flex items-center space-x-3">
              <ExclamationTriangleIcon className="h-6 w-6 text-red-600" />
              <div>
                <p className="text-sm font-medium text-red-800">
                  Failed to load dashboard data
                </p>
                <p className="text-sm text-red-600 mt-1">
                  {error?.message || 'An unexpected error occurred'}
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Alerts summary */}
        {alertsQuery.data && alertsQuery.data.length > 0 && (
          <div className="mb-6 bg-yellow-50 border border-yellow-200 rounded-lg p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <BellAlertIcon className="h-6 w-6 text-yellow-600" />
                <div>
                  <p className="text-sm font-medium text-yellow-800">
                    You have {alertsQuery.data.length} active alert
                    {alertsQuery.data.length !== 1 ? 's' : ''}
                  </p>
                  <p className="text-sm text-yellow-600 mt-1">
                    Review and acknowledge pending alerts
                  </p>
                </div>
              </div>
              <button
                type="button"
                className="px-4 py-2 bg-yellow-600 text-white text-sm font-medium rounded-lg hover:bg-yellow-700 transition-colors"
              >
                View Alerts
              </button>
            </div>
          </div>
        )}

        {/* Key Metrics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {/* Queries per second */}
          <MetricCard
            icon={ChartBarIcon}
            label="Queries per Second"
            value={database?.queriesPerSecond.toFixed(1) || '0'}
            unit="q/s"
            change={5.2}
            sparklineData={
              getSparklineData(performanceQuery.data?.metrics.queriesPerSecond)
            }
            status="normal"
            isLoading={isLoading}
          />

          {/* Active connections */}
          <MetricCard
            icon={SignalIcon}
            label="Active Connections"
            value={database?.activeConnections || 0}
            unit={`/ ${database?.maxConnections || 0}`}
            change={-2.1}
            sparklineData={
              getSparklineData(performanceQuery.data?.metrics.activeConnections)
            }
            status={
              database && database.activeConnections > database.maxConnections * 0.8
                ? 'warning'
                : 'normal'
            }
            isLoading={isLoading}
          />

          {/* Cache hit ratio */}
          <MetricCard
            icon={BoltIcon}
            label="Cache Hit Ratio"
            value={formatPercentage(database?.cacheHitRatio || 0)}
            change={1.8}
            sparklineData={
              getSparklineData(performanceQuery.data?.metrics.cacheHitRatio)
            }
            status={
              database && database.cacheHitRatio < 80 ? 'warning' : 'success'
            }
            isLoading={isLoading}
          />

          {/* CPU usage */}
          <MetricCard
            icon={CpuChipIcon}
            label="CPU Usage"
            value={formatPercentage(system?.cpu.usage || 0)}
            change={-3.5}
            sparklineData={getSparklineData(performanceQuery.data?.metrics.cpuUsage)}
            status={
              system && system.cpu.usage > 80
                ? 'critical'
                : system && system.cpu.usage > 60
                ? 'warning'
                : 'normal'
            }
            isLoading={isLoading}
          />

          {/* Memory usage */}
          <MetricCard
            icon={ServerIcon}
            label="Memory Usage"
            value={system ? formatBytes(system.memory.used) : '0 MB'}
            unit={system ? `/ ${formatBytes(system.memory.total)}` : ''}
            change={0.8}
            sparklineData={
              getSparklineData(performanceQuery.data?.metrics.memoryUsage)
            }
            status={
              system && system.memory.usagePercent > 85
                ? 'warning'
                : 'normal'
            }
            isLoading={isLoading}
          />

          {/* Disk usage */}
          <MetricCard
            icon={CircleStackIcon}
            label="Disk Usage"
            value={system ? formatBytes(system.disk.used) : '0 GB'}
            unit={system ? `/ ${formatBytes(system.disk.total)}` : ''}
            change={2.3}
            sparklineData={getSparklineData(performanceQuery.data?.metrics.diskUsage)}
            status={
              system && system.disk.usagePercent > 90
                ? 'critical'
                : system && system.disk.usagePercent > 75
                ? 'warning'
                : 'normal'
            }
            isLoading={isLoading}
          />

          {/* Transactions per second */}
          <MetricCard
            icon={ArrowPathIcon}
            label="Transactions per Second"
            value={database?.transactionsPerSecond.toFixed(1) || '0'}
            unit="tx/s"
            change={4.7}
            status="normal"
            isLoading={isLoading}
          />

          {/* Average response time */}
          <MetricCard
            icon={ClockIcon}
            label="Avg Response Time"
            value={queries?.avgExecutionTime.toFixed(0) || '0'}
            unit="ms"
            change={-8.2}
            sparklineData={
              getSparklineData(performanceQuery.data?.metrics.avgResponseTime)
            }
            status={
              queries && queries.avgExecutionTime > 1000
                ? 'warning'
                : 'success'
            }
            isLoading={isLoading}
          />
        </div>

        {/* Charts Row */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          {/* Performance Chart */}
          <PerformanceChart
            title="Query Performance"
            series={performanceChartSeries}
            timeRange={timeRange}
            onTimeRangeChange={setTimeRange}
            chartType="area"
            height={300}
            isLoading={performanceQuery.isLoading}
            error={performanceQuery.error?.message}
          />

          {/* Resource Usage Chart */}
          <PerformanceChart
            title="Resource Usage"
            series={resourceChartSeries}
            timeRange={timeRange}
            onTimeRangeChange={setTimeRange}
            chartType="line"
            height={300}
            isLoading={performanceQuery.isLoading}
            error={performanceQuery.error?.message}
          />
        </div>

        {/* Health & Activity Row */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* System Health */}
          {health && (
            <div className="lg:col-span-1">
              <HealthIndicator
                health={health}
                showComponents={true}
                showUptime={true}
                showVersion={true}
                isLoading={isLoading}
              />
            </div>
          )}

          {/* Activity Feed */}
          <div className="lg:col-span-2">
            <ActivityFeed
              events={activityQuery.data?.events || []}
              isLoading={activityQuery.isLoading}
              error={activityQuery.error?.message}
              onLoadMore={
                activityQuery.hasMore
                  ? () => {
                      // Load more functionality would be implemented here
                      console.log('Load more activities');
                    }
                  : undefined
              }
              hasMore={activityQuery.hasMore}
              maxHeight="600px"
              showFilter={true}
              showTimestamps={true}
              realtime={realtimeEnabled}
            />
          </div>
        </div>

        {/* Quick Stats Footer */}
        <div className="mt-8 bg-white rounded-lg border border-gray-200 p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Database Overview
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
            <div>
              <p className="text-sm text-gray-600">Total Queries</p>
              <p className="mt-1 text-2xl font-bold text-gray-900">
                {queries?.totalQueries.toLocaleString() || '0'}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Active Queries</p>
              <p className="mt-1 text-2xl font-bold text-gray-900">
                {queries?.activeQueries || 0}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Slow Queries</p>
              <p className="mt-1 text-2xl font-bold text-gray-900">
                {queries?.slowQueries || 0}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Failed Queries</p>
              <p className="mt-1 text-2xl font-bold text-red-600">
                {queries?.failedQueries || 0}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// Export
// ============================================================================

export default Dashboard;
