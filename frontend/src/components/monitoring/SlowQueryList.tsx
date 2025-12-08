import { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { formatDistanceToNow } from 'date-fns';
import type { SlowQuery, UUID } from '../../types';

interface SlowQueryListProps {
  queries: SlowQuery[];
  onViewExplain?: (queryId: UUID) => void;
  className?: string;
}

export function SlowQueryList({
  queries,
  onViewExplain,
  className = '',
}: SlowQueryListProps) {
  const [expandedQuery, setExpandedQuery] = useState<UUID | null>(null);
  const [sortBy, setSortBy] = useState<'executionTime' | 'timestamp'>('executionTime');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [filter, setFilter] = useState({
    database: '',
    user: '',
    minDuration: '',
    search: '',
  });

  const filteredQueries = useMemo(() => {
    return queries.filter((query) => {
      if (filter.database && query.database !== filter.database) return false;
      if (filter.user && query.userId && !query.userId.includes(filter.user)) return false;
      if (filter.minDuration && query.executionTime < parseFloat(filter.minDuration)) return false;
      if (filter.search && !query.sql.toLowerCase().includes(filter.search.toLowerCase())) return false;
      return true;
    });
  }, [queries, filter]);

  const sortedQueries = useMemo(() => {
    return [...filteredQueries].sort((a, b) => {
      const aVal = a[sortBy];
      const bVal = b[sortBy];

      let comparison = 0;
      if (typeof aVal === 'number' && typeof bVal === 'number') {
        comparison = aVal - bVal;
      } else {
        comparison = String(aVal).localeCompare(String(bVal));
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });
  }, [filteredQueries, sortBy, sortOrder]);

  const handleSort = (column: 'executionTime' | 'timestamp') => {
    if (sortBy === column) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(column);
      setSortOrder('desc');
    }
  };

  const toggleExpanded = (queryId: UUID) => {
    setExpandedQuery(expandedQuery === queryId ? null : queryId);
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms.toFixed(0)}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(2)}s`;
    return `${(ms / 60000).toFixed(2)}m`;
  };

  const getDurationColor = (ms: number) => {
    if (ms >= 10000) return 'text-red-400';
    if (ms >= 5000) return 'text-orange-400';
    if (ms >= 1000) return 'text-yellow-400';
    return 'text-blue-400';
  };

  // Query fingerprinting - normalize query for grouping
  const getQueryFingerprint = (sql: string): string => {
    return sql
      .replace(/\d+/g, '?')
      .replace(/'[^']*'/g, '?')
      .replace(/\s+/g, ' ')
      .trim()
      .substring(0, 100);
  };

  const uniqueDatabases = [...new Set(queries.map(q => q.database))];
  const uniqueUsers = [...new Set(queries.map(q => q.userId).filter(Boolean))];

  // Calculate execution time histogram
  const timeRanges = [
    { label: '< 1s', min: 0, max: 1000 },
    { label: '1-5s', min: 1000, max: 5000 },
    { label: '5-10s', min: 5000, max: 10000 },
    { label: '> 10s', min: 10000, max: Infinity },
  ];

  const histogram = timeRanges.map(range => ({
    ...range,
    count: filteredQueries.filter(q => q.executionTime >= range.min && q.executionTime < range.max).length,
  }));

  return (
    <div className={className}>
      {/* Filters */}
      <div className="mb-4 grid grid-cols-1 md:grid-cols-4 gap-3">
        <input
          type="text"
          placeholder="Search SQL..."
          value={filter.search}
          onChange={(e) => setFilter({ ...filter, search: e.target.value })}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />

        <select
          value={filter.database}
          onChange={(e) => setFilter({ ...filter, database: e.target.value })}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All Databases</option>
          {uniqueDatabases.map(db => (
            <option key={db} value={db}>{db}</option>
          ))}
        </select>

        <select
          value={filter.user}
          onChange={(e) => setFilter({ ...filter, user: e.target.value })}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All Users</option>
          {uniqueUsers.map(user => (
            <option key={user} value={user}>{user}</option>
          ))}
        </select>

        <input
          type="number"
          placeholder="Min duration (ms)"
          value={filter.minDuration}
          onChange={(e) => setFilter({ ...filter, minDuration: e.target.value })}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Histogram */}
      <div className="mb-4 p-4 bg-gray-800 rounded-lg">
        <h3 className="text-sm font-medium text-gray-300 mb-3">Execution Time Distribution</h3>
        <div className="grid grid-cols-4 gap-3">
          {histogram.map(({ label, count }) => (
            <div key={label} className="text-center">
              <div className="text-2xl font-bold text-blue-400">{count}</div>
              <div className="text-xs text-gray-400">{label}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Sort Controls */}
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center space-x-4 text-sm text-gray-400">
          <span>Total: {sortedQueries.length} queries</span>
        </div>
        <div className="flex items-center space-x-2">
          <span className="text-sm text-gray-400">Sort by:</span>
          <button
            onClick={() => handleSort('executionTime')}
            className={`px-3 py-1 text-sm rounded ${
              sortBy === 'executionTime'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            Duration {sortBy === 'executionTime' && (sortOrder === 'asc' ? '↑' : '↓')}
          </button>
          <button
            onClick={() => handleSort('timestamp')}
            className={`px-3 py-1 text-sm rounded ${
              sortBy === 'timestamp'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            Time {sortBy === 'timestamp' && (sortOrder === 'asc' ? '↑' : '↓')}
          </button>
        </div>
      </div>

      {/* Query List */}
      <div className="space-y-3">
        <AnimatePresence>
          {sortedQueries.map((query) => (
            <motion.div
              key={query.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, x: -100 }}
              className="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden"
            >
              <div
                className="p-4 cursor-pointer hover:bg-gray-750 transition-colors"
                onClick={() => toggleExpanded(query.id)}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center space-x-3 mb-1">
                      <span className={`text-lg font-bold ${getDurationColor(query.executionTime)}`}>
                        {formatDuration(query.executionTime)}
                      </span>
                      <span className="text-xs px-2 py-1 bg-gray-700 rounded text-gray-300">
                        {query.database}
                      </span>
                      {query.userId && (
                        <span className="text-xs px-2 py-1 bg-gray-700 rounded text-gray-300">
                          {query.userId}
                        </span>
                      )}
                      <span className="text-xs text-gray-500">
                        {formatDistanceToNow(new Date(query.timestamp), { addSuffix: true })}
                      </span>
                    </div>
                    <code className="text-sm text-gray-300 block truncate">
                      {query.sql.substring(0, 150)}
                      {query.sql.length > 150 && '...'}
                    </code>
                    {query.rowsAffected !== undefined && (
                      <div className="text-xs text-gray-500 mt-1">
                        Rows affected: {query.rowsAffected}
                      </div>
                    )}
                  </div>
                  <div className="flex items-center space-x-2 ml-4">
                    {onViewExplain && query.explain && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onViewExplain(query.id);
                        }}
                        className="px-3 py-1 text-xs font-medium text-blue-400 hover:text-blue-300 hover:bg-blue-900 hover:bg-opacity-20 rounded transition-colors"
                      >
                        View Plan
                      </button>
                    )}
                    <span className="text-gray-500">
                      {expandedQuery === query.id ? '▼' : '▶'}
                    </span>
                  </div>
                </div>
              </div>

              {expandedQuery === query.id && (
                <motion.div
                  initial={{ height: 0 }}
                  animate={{ height: 'auto' }}
                  exit={{ height: 0 }}
                  className="border-t border-gray-700"
                >
                  <div className="p-4 bg-gray-900">
                    <h4 className="text-sm font-medium text-gray-300 mb-2">Full Query</h4>
                    <pre className="text-xs text-gray-300 bg-black bg-opacity-50 p-3 rounded overflow-x-auto">
                      {query.sql}
                    </pre>

                    <div className="mt-3 text-xs text-gray-500">
                      <div>Fingerprint: {getQueryFingerprint(query.sql)}</div>
                    </div>

                    {query.explain && (
                      <div className="mt-4">
                        <h4 className="text-sm font-medium text-gray-300 mb-2">Execution Plan Summary</h4>
                        <div className="grid grid-cols-2 gap-3 text-xs">
                          <div>
                            <span className="text-gray-500">Planning Time:</span>
                            <span className="ml-2 text-gray-300">{query.explain.planningTime}ms</span>
                          </div>
                          <div>
                            <span className="text-gray-500">Execution Time:</span>
                            <span className="ml-2 text-gray-300">{query.explain.executionTime}ms</span>
                          </div>
                          <div>
                            <span className="text-gray-500">Total Cost:</span>
                            <span className="ml-2 text-gray-300">{query.explain.totalCost.toFixed(2)}</span>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </motion.div>
              )}
            </motion.div>
          ))}
        </AnimatePresence>
      </div>

      {sortedQueries.length === 0 && (
        <div className="p-8 text-center text-gray-500 bg-gray-800 rounded-lg">
          No slow queries found matching your filters
        </div>
      )}
    </div>
  );
}
