import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Link } from 'react-router-dom';
import { useAlerts } from '../hooks/useMonitoring';
import { AlertCard } from '../components/monitoring/AlertCard';
import type { AlertSeverity, AlertType } from '../types';

export default function Alerts() {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [activeTab, setActiveTab] = useState<'active' | 'history'>('active');
  const [filters, setFilters] = useState<{
    severity?: string;
    type?: string;
    acknowledged?: boolean;
    resolved?: boolean;
  }>({
    resolved: false,
  });

  const { data, isLoading, error, refresh, acknowledgeAlert, resolveAlert } = useAlerts(
    {
      page,
      pageSize,
      sortBy: 'timestamp',
      sortOrder: 'desc',
      ...filters,
    },
    true
  );

  const handleAcknowledge = async (alertId: string, note?: string) => {
    try {
      await acknowledgeAlert(alertId, note);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to acknowledge alert');
    }
  };

  const handleResolve = async (alertId: string, resolution?: string) => {
    try {
      await resolveAlert(alertId, resolution);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to resolve alert');
    }
  };

  const handleTabChange = (tab: 'active' | 'history') => {
    setActiveTab(tab);
    if (tab === 'active') {
      setFilters({ resolved: false });
    } else {
      setFilters({ resolved: true });
    }
    setPage(1);
  };

  const criticalCount = data?.data.filter((a) => a.severity === 'critical' && !a.resolved).length || 0;
  const errorCount = data?.data.filter((a) => a.severity === 'error' && !a.resolved).length || 0;
  const warningCount = data?.data.filter((a) => a.severity === 'warning' && !a.resolved).length || 0;
  const unacknowledgedCount = data?.data.filter((a) => !a.acknowledged && !a.resolved).length || 0;

  const severityOptions: AlertSeverity[] = ['critical', 'error', 'warning', 'info'];
  const typeOptions: AlertType[] = [
    'performance',
    'security',
    'availability',
    'capacity',
    'replication',
    'backup',
    'configuration',
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center space-x-3">
            <Link
              to="/monitoring"
              className="text-gray-400 hover:text-white transition-colors"
            >
              ← Back to Monitoring
            </Link>
          </div>
          <h1 className="text-3xl font-bold text-white mt-2">Alert Management</h1>
          <p className="text-gray-400 mt-1">
            Monitor and respond to system alerts and notifications
          </p>
        </div>
        <div className="flex items-center space-x-3">
          <button
            onClick={refresh}
            disabled={isLoading}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:opacity-50 flex items-center space-x-2"
          >
            <span>🔄</span>
            <span>{isLoading ? 'Loading...' : 'Refresh'}</span>
          </button>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-red-900 bg-opacity-30 border border-red-500 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-red-300">Critical</h3>
            <span className="text-2xl">🔴</span>
          </div>
          <div className="text-3xl font-bold text-red-400">{criticalCount}</div>
          <div className="text-xs text-red-300 mt-1">Immediate attention required</div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-orange-900 bg-opacity-30 border border-orange-500 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-orange-300">Error</h3>
            <span className="text-2xl">❌</span>
          </div>
          <div className="text-3xl font-bold text-orange-400">{errorCount}</div>
          <div className="text-xs text-orange-300 mt-1">Errors requiring action</div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-yellow-900 bg-opacity-30 border border-yellow-500 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-yellow-300">Warning</h3>
            <span className="text-2xl">⚠️</span>
          </div>
          <div className="text-3xl font-bold text-yellow-400">{warningCount}</div>
          <div className="text-xs text-yellow-300 mt-1">Warnings to review</div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-blue-900 bg-opacity-30 border border-blue-500 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-blue-300">Unacknowledged</h3>
            <span className="text-2xl">🔔</span>
          </div>
          <div className="text-3xl font-bold text-blue-400">{unacknowledgedCount}</div>
          <div className="text-xs text-blue-300 mt-1">Need acknowledgment</div>
        </motion.div>
      </div>

      {/* Tabs */}
      <div className="flex items-center space-x-2 border-b border-gray-700">
        <button
          onClick={() => handleTabChange('active')}
          className={`px-4 py-2 font-medium transition-colors ${
            activeTab === 'active'
              ? 'text-blue-400 border-b-2 border-blue-400'
              : 'text-gray-400 hover:text-gray-300'
          }`}
        >
          Active Alerts ({data?.data.filter((a) => !a.resolved).length || 0})
        </button>
        <button
          onClick={() => handleTabChange('history')}
          className={`px-4 py-2 font-medium transition-colors ${
            activeTab === 'history'
              ? 'text-blue-400 border-b-2 border-blue-400'
              : 'text-gray-400 hover:text-gray-300'
          }`}
        >
          History
        </button>
      </div>

      {/* Filters */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.4 }}
        className="bg-gray-800 rounded-lg p-6"
      >
        <h3 className="text-lg font-semibold text-white mb-4">Filters</h3>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
          <select
            value={filters.severity || ''}
            onChange={(e) => {
              setFilters({ ...filters, severity: e.target.value || undefined });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">All Severities</option>
            {severityOptions.map((severity) => (
              <option key={severity} value={severity}>
                {severity.charAt(0).toUpperCase() + severity.slice(1)}
              </option>
            ))}
          </select>

          <select
            value={filters.type || ''}
            onChange={(e) => {
              setFilters({ ...filters, type: e.target.value || undefined });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">All Types</option>
            {typeOptions.map((type) => (
              <option key={type} value={type}>
                {type.charAt(0).toUpperCase() + type.slice(1)}
              </option>
            ))}
          </select>

          {activeTab === 'active' && (
            <select
              value={filters.acknowledged === undefined ? '' : filters.acknowledged ? 'true' : 'false'}
              onChange={(e) => {
                const value = e.target.value === '' ? undefined : e.target.value === 'true';
                setFilters({ ...filters, acknowledged: value });
                setPage(1);
              }}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">All Alerts</option>
              <option value="false">Unacknowledged Only</option>
              <option value="true">Acknowledged Only</option>
            </select>
          )}

          <button
            onClick={() => {
              setFilters(activeTab === 'active' ? { resolved: false } : { resolved: true });
              setPage(1);
            }}
            className="px-3 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm transition-colors"
          >
            Clear Filters
          </button>
        </div>
      </motion.div>

      {/* Error Display */}
      {error && (
        <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
          <p className="text-red-400">Error loading alerts: {error}</p>
        </div>
      )}

      {/* Alerts List */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.5 }}
        className="space-y-3"
      >
        {isLoading && !data ? (
          <div className="bg-gray-800 rounded-lg p-8 text-center">
            <div className="text-gray-400">Loading alerts...</div>
          </div>
        ) : data && data.data.length > 0 ? (
          <>
            <AnimatePresence>
              {data.data.map((alert) => (
                <AlertCard
                  key={alert.id}
                  alert={alert}
                  onAcknowledge={activeTab === 'active' ? handleAcknowledge : undefined}
                  onResolve={activeTab === 'active' ? handleResolve : undefined}
                />
              ))}
            </AnimatePresence>

            {/* Pagination */}
            {data.totalPages > 1 && (
              <div className="mt-6 flex items-center justify-between bg-gray-800 rounded-lg p-4">
                <div className="text-sm text-gray-400">
                  Showing {(page - 1) * pageSize + 1} to{' '}
                  {Math.min(page * pageSize, data.total)} of {data.total} alerts
                </div>
                <div className="flex items-center space-x-2">
                  <button
                    onClick={() => setPage(page - 1)}
                    disabled={!data.hasPrevious}
                    className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    Previous
                  </button>
                  <span className="text-sm text-gray-400">
                    Page {page} of {data.totalPages}
                  </span>
                  <button
                    onClick={() => setPage(page + 1)}
                    disabled={!data.hasNext}
                    className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    Next
                  </button>
                </div>
              </div>
            )}
          </>
        ) : (
          <div className="bg-gray-800 rounded-lg p-8 text-center">
            <div className="text-green-400 text-5xl mb-4">✓</div>
            <h3 className="text-lg font-medium text-gray-300 mb-2">No Alerts</h3>
            <p className="text-sm text-gray-500">
              {activeTab === 'active'
                ? 'All systems are operating normally'
                : 'No resolved alerts in history'}
            </p>
          </div>
        )}
      </motion.div>

      {/* Alert Configuration Link */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.7 }}
        className="bg-blue-900 bg-opacity-20 border border-blue-500 border-opacity-30 rounded-lg p-4"
      >
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-blue-300 mb-1">Alert Rules</h3>
            <p className="text-xs text-blue-200">
              Configure alert thresholds and notification settings
            </p>
          </div>
          <button className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm transition-colors">
            Configure Rules
          </button>
        </div>
      </motion.div>

      {/* Help Text */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.8 }}
        className="bg-gray-800 rounded-lg p-4"
      >
        <h3 className="text-sm font-semibold text-gray-300 mb-2">About Alert Severity</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs text-gray-400">
          <div>
            <span className="font-medium text-red-400">Critical:</span> System-wide issues requiring
            immediate attention. May indicate service downtime or data loss.
          </div>
          <div>
            <span className="font-medium text-orange-400">Error:</span> Significant problems that need
            prompt resolution but aren't causing immediate outage.
          </div>
          <div>
            <span className="font-medium text-yellow-400">Warning:</span> Potential issues that should
            be investigated but aren't currently critical.
          </div>
          <div>
            <span className="font-medium text-blue-400">Info:</span> Informational messages about
            system events and routine operations.
          </div>
        </div>
      </motion.div>
    </div>
  );
}
