import { useState } from 'react';
import { motion } from 'framer-motion';
import { Link } from 'react-router-dom';
import { useSlowQueries } from '../hooks/useMonitoring';
import { SlowQueryList } from '../components/monitoring/SlowQueryList';
import type { UUID } from '../types';

export default function SlowQueries() {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [filters, setFilters] = useState({
    database: '',
    user: '',
    minDuration: '',
    startDate: '',
    endDate: '',
    search: '',
  });
  const [showExplainModal, setShowExplainModal] = useState<UUID | null>(null);

  const { data, isLoading, error, refresh, exportQueries } = useSlowQueries({
    page,
    pageSize,
    sortBy: 'executionTime',
    sortOrder: 'desc',
    ...filters,
  });

  const handleExport = async (format: 'json' | 'csv') => {
    try {
      await exportQueries(format);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to export queries');
    }
  };

  const handleViewExplain = (queryId: UUID) => {
    setShowExplainModal(queryId);
  };

  const avgExecutionTime =
    data && data.data.length > 0
      ? data.data.reduce((sum, q) => sum + q.executionTime, 0) / data.data.length
      : 0;

  const maxExecutionTime =
    data && data.data.length > 0
      ? Math.max(...data.data.map((q) => q.executionTime))
      : 0;

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms.toFixed(0)}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(2)}s`;
    return `${(ms / 60000).toFixed(2)}m`;
  };

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
          <h1 className="text-3xl font-bold text-white mt-2">Slow Query Analysis</h1>
          <p className="text-gray-400 mt-1">
            Identify and optimize slow-performing queries
          </p>
        </div>
        <div className="flex items-center space-x-3">
          <button
            onClick={() => handleExport('csv')}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors flex items-center space-x-2"
          >
            <span>📥</span>
            <span>Export CSV</span>
          </button>
          <button
            onClick={() => handleExport('json')}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors flex items-center space-x-2"
          >
            <span>📥</span>
            <span>Export JSON</span>
          </button>
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
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Total Slow Queries</h3>
            <span className="text-2xl">🐌</span>
          </div>
          <div className="text-3xl font-bold text-white">{data?.total || 0}</div>
          <div className="text-xs text-gray-500 mt-1">
            Showing {data?.data.length || 0} on this page
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Avg Execution Time</h3>
            <span className="text-2xl">⏱️</span>
          </div>
          <div className="text-3xl font-bold text-yellow-400">
            {formatDuration(avgExecutionTime)}
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Max Execution Time</h3>
            <span className="text-2xl">🔴</span>
          </div>
          <div className="text-3xl font-bold text-red-400">
            {formatDuration(maxExecutionTime)}
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Page Size</h3>
            <span className="text-2xl">📄</span>
          </div>
          <select
            value={pageSize}
            onChange={(e) => {
              setPageSize(Number(e.target.value));
              setPage(1);
            }}
            className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value={10}>10 per page</option>
            <option value={20}>20 per page</option>
            <option value={50}>50 per page</option>
            <option value={100}>100 per page</option>
          </select>
        </motion.div>
      </div>

      {/* Advanced Filters */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.4 }}
        className="bg-gray-800 rounded-lg p-6"
      >
        <h3 className="text-lg font-semibold text-white mb-4">Filters</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-3">
          <input
            type="text"
            placeholder="Database..."
            value={filters.database}
            onChange={(e) => {
              setFilters({ ...filters, database: e.target.value });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <input
            type="text"
            placeholder="User..."
            value={filters.user}
            onChange={(e) => {
              setFilters({ ...filters, user: e.target.value });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <input
            type="number"
            placeholder="Min duration (ms)..."
            value={filters.minDuration}
            onChange={(e) => {
              setFilters({ ...filters, minDuration: e.target.value });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <input
            type="datetime-local"
            placeholder="Start date..."
            value={filters.startDate}
            onChange={(e) => {
              setFilters({ ...filters, startDate: e.target.value });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <input
            type="datetime-local"
            placeholder="End date..."
            value={filters.endDate}
            onChange={(e) => {
              setFilters({ ...filters, endDate: e.target.value });
              setPage(1);
            }}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <button
            onClick={() => {
              setFilters({
                database: '',
                user: '',
                minDuration: '',
                startDate: '',
                endDate: '',
                search: '',
              });
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
          <p className="text-red-400">Error loading slow queries: {error}</p>
        </div>
      )}

      {/* Slow Query List */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.5 }}
      >
        {isLoading && !data ? (
          <div className="bg-gray-800 rounded-lg p-8 text-center">
            <div className="text-gray-400">Loading slow queries...</div>
          </div>
        ) : data ? (
          <>
            <SlowQueryList
              queries={data.data}
              onViewExplain={handleViewExplain}
            />

            {/* Pagination */}
            {data.totalPages > 1 && (
              <div className="mt-6 flex items-center justify-between">
                <div className="text-sm text-gray-400">
                  Showing {(page - 1) * pageSize + 1} to{' '}
                  {Math.min(page * pageSize, data.total)} of {data.total} queries
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
            <div className="text-gray-400">No slow queries found</div>
          </div>
        )}
      </motion.div>

      {/* Optimization Tips */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.7 }}
        className="bg-blue-900 bg-opacity-20 border border-blue-500 border-opacity-30 rounded-lg p-4"
      >
        <h3 className="text-sm font-semibold text-blue-300 mb-2">Query Optimization Tips</h3>
        <ul className="text-xs text-blue-200 space-y-1 list-disc list-inside">
          <li>Add appropriate indexes on columns used in WHERE, JOIN, and ORDER BY clauses</li>
          <li>Avoid SELECT * - only fetch columns you need</li>
          <li>Use LIMIT to restrict result sets when possible</li>
          <li>Consider query caching for frequently executed queries</li>
          <li>Review execution plans to identify bottlenecks</li>
          <li>Denormalize data or use materialized views for complex aggregations</li>
        </ul>
      </motion.div>
    </div>
  );
}
