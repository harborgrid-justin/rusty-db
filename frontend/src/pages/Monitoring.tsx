import { motion } from 'framer-motion';
import { Link } from 'react-router-dom';
import { LineChart, Line, AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { format } from 'date-fns';
import { useCurrentMetrics, usePerformanceMetrics } from '../hooks/useMonitoring';
import { PerformanceGauges } from '../components/monitoring/PerformanceGauges';
import type { SystemMetrics } from '../types';

export default function Monitoring() {
  const { metrics, isLoading: metricsLoading, error: metricsError, refresh } = useCurrentMetrics(true);
  const { data: performanceData, isLoading: perfLoading } = usePerformanceMetrics(60, 30000);

  if (metricsLoading || perfLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading monitoring data...</div>
      </div>
    );
  }

  if (metricsError) {
    return (
      <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
        <p className="text-red-400">Error loading metrics: {metricsError}</p>
      </div>
    );
  }

  if (!metrics) {
    return (
      <div className="bg-gray-800 rounded-lg p-8 text-center">
        <p className="text-gray-400">No metrics available</p>
      </div>
    );
  }

  // Transform history data for charts
  const chartData = performanceData?.history.map((m: SystemMetrics) => ({
    timestamp: format(new Date(m.timestamp), 'HH:mm'),
    cpu: m.cpu.usage,
    memory: m.memory.usagePercent,
    disk: m.disk.usagePercent,
    connections: m.database.activeConnections,
    qps: m.database.queriesPerSecond,
    tps: m.database.transactionsPerSecond,
  })) || [];

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(2)} GB`;
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">System Monitoring</h1>
          <p className="text-gray-400 mt-1">Real-time performance and health metrics</p>
        </div>
        <div className="flex items-center space-x-3">
          <button
            onClick={refresh}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors flex items-center space-x-2"
          >
            <span>🔄</span>
            <span>Refresh</span>
          </button>
          <Link
            to="/monitoring/sessions"
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
          >
            View Sessions
          </Link>
        </div>
      </div>

      {/* Performance Gauges */}
      <PerformanceGauges metrics={metrics} />

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Active Connections</h3>
            <span className="text-2xl">🔌</span>
          </div>
          <div className="text-3xl font-bold text-white">
            {metrics.database.activeConnections}
          </div>
          <div className="text-xs text-gray-500 mt-1">
            of {metrics.database.maxConnections} max
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Queries/Second</h3>
            <span className="text-2xl">⚡</span>
          </div>
          <div className="text-3xl font-bold text-white">
            {metrics.database.queriesPerSecond.toFixed(1)}
          </div>
          <div className="text-xs text-gray-500 mt-1">
            {metrics.database.transactionsPerSecond.toFixed(1)} TPS
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Cache Hit Ratio</h3>
            <span className="text-2xl">📊</span>
          </div>
          <div className="text-3xl font-bold text-white">
            {metrics.database.cacheHitRatio.toFixed(1)}%
          </div>
          <div className="text-xs text-gray-500 mt-1">
            Buffer pool efficiency
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Lock Waits</h3>
            <span className="text-2xl">🔒</span>
          </div>
          <div className="text-3xl font-bold text-white">
            {metrics.database.lockWaits}
          </div>
          <div className="text-xs text-gray-500 mt-1">
            {metrics.database.deadlocks} deadlocks
          </div>
        </motion.div>
      </div>

      {/* Resource Usage Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">Resource Usage</h3>
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="timestamp" stroke="#9CA3AF" style={{ fontSize: 12 }} />
              <YAxis stroke="#9CA3AF" style={{ fontSize: 12 }} />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#1F2937',
                  border: '1px solid #374151',
                  borderRadius: '0.5rem',
                }}
              />
              <Legend />
              <Area
                type="monotone"
                dataKey="cpu"
                stackId="1"
                stroke="#3B82F6"
                fill="#3B82F6"
                name="CPU %"
              />
              <Area
                type="monotone"
                dataKey="memory"
                stackId="2"
                stroke="#10B981"
                fill="#10B981"
                name="Memory %"
              />
              <Area
                type="monotone"
                dataKey="disk"
                stackId="3"
                stroke="#F59E0B"
                fill="#F59E0B"
                name="Disk %"
              />
            </AreaChart>
          </ResponsiveContainer>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">Database Activity</h3>
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="timestamp" stroke="#9CA3AF" style={{ fontSize: 12 }} />
              <YAxis stroke="#9CA3AF" style={{ fontSize: 12 }} />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#1F2937',
                  border: '1px solid #374151',
                  borderRadius: '0.5rem',
                }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="qps"
                stroke="#8B5CF6"
                strokeWidth={2}
                name="Queries/sec"
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="tps"
                stroke="#EC4899"
                strokeWidth={2}
                name="Transactions/sec"
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="connections"
                stroke="#06B6D4"
                strokeWidth={2}
                name="Active Connections"
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </motion.div>
      </div>

      {/* System Details */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.6 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">CPU Details</h3>
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Cores:</span>
              <span className="text-white font-medium">{metrics.cpu.cores}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">User Time:</span>
              <span className="text-white font-medium">{metrics.cpu.userTime.toFixed(2)}%</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">System Time:</span>
              <span className="text-white font-medium">{metrics.cpu.systemTime.toFixed(2)}%</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Idle Time:</span>
              <span className="text-white font-medium">{metrics.cpu.idleTime.toFixed(2)}%</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Load Average:</span>
              <span className="text-white font-medium">
                {metrics.cpu.loadAverage.map(l => l.toFixed(2)).join(', ')}
              </span>
            </div>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.7 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">Memory Details</h3>
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Total:</span>
              <span className="text-white font-medium">{formatBytes(metrics.memory.total)}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Used:</span>
              <span className="text-white font-medium">{formatBytes(metrics.memory.used)}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Free:</span>
              <span className="text-white font-medium">{formatBytes(metrics.memory.free)}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Cached:</span>
              <span className="text-white font-medium">{formatBytes(metrics.memory.cached)}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Buffers:</span>
              <span className="text-white font-medium">{formatBytes(metrics.memory.buffers)}</span>
            </div>
          </div>
        </motion.div>
      </div>

      {/* Quick Links */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.8 }}
        className="bg-gray-800 rounded-lg p-6"
      >
        <h3 className="text-lg font-semibold text-white mb-4">Monitoring Tools</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Link
            to="/monitoring/sessions"
            className="p-4 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
          >
            <div className="text-2xl mb-2">👥</div>
            <h4 className="font-medium text-white mb-1">Active Sessions</h4>
            <p className="text-xs text-gray-400">View and manage database sessions</p>
          </Link>

          <Link
            to="/monitoring/slow-queries"
            className="p-4 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
          >
            <div className="text-2xl mb-2">🐌</div>
            <h4 className="font-medium text-white mb-1">Slow Queries</h4>
            <p className="text-xs text-gray-400">Analyze query performance issues</p>
          </Link>

          <Link
            to="/monitoring/alerts"
            className="p-4 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
          >
            <div className="text-2xl mb-2">🔔</div>
            <h4 className="font-medium text-white mb-1">Alerts</h4>
            <p className="text-xs text-gray-400">Monitor and manage system alerts</p>
          </Link>
        </div>
      </motion.div>
    </div>
  );
}
