import { useState } from 'react';
import { motion } from 'framer-motion';
import { Link } from 'react-router-dom';
import { useActiveSessions, useBlockingTree } from '../hooks/useMonitoring';
import { SessionTable } from '../components/monitoring/SessionTable';
import { BlockingTree } from '../components/monitoring/BlockingTree';

export default function Sessions() {
  const [showBlocking, setShowBlocking] = useState(false);
  const [refreshInterval, setRefreshInterval] = useState(10000);

  const { sessions, isLoading, error, refresh, killSession } = useActiveSessions(
    undefined,
    refreshInterval
  );

  const {
    data: blockingData,
    isLoading: blockingLoading,
    refresh: refreshBlocking,
  } = useBlockingTree(15000);

  const handleKillSession = async (sessionId: string, force: boolean) => {
    await killSession(sessionId, force);
  };

  const activeCount = sessions.filter((s) => s.state === 'active').length;
  const idleCount = sessions.filter((s) => s.state === 'idle').length;
  const blockedCount = sessions.filter((s) => s.blockedBy).length;
  const blockingCount = blockingData?.nodes.filter((n) => n.blocking.length > 0).length || 0;

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
          <h1 className="text-3xl font-bold text-white mt-2">Active Sessions</h1>
          <p className="text-gray-400 mt-1">
            Monitor and manage database sessions in real-time
          </p>
        </div>
        <div className="flex items-center space-x-3">
          <select
            value={refreshInterval}
            onChange={(e) => setRefreshInterval(Number(e.target.value))}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value={5000}>Refresh: 5s</option>
            <option value={10000}>Refresh: 10s</option>
            <option value={30000}>Refresh: 30s</option>
            <option value={60000}>Refresh: 1m</option>
          </select>
          <button
            onClick={refresh}
            disabled={isLoading}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors disabled:opacity-50 flex items-center space-x-2"
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
            <h3 className="text-sm font-medium text-gray-400">Total Sessions</h3>
            <span className="text-2xl">📊</span>
          </div>
          <div className="text-3xl font-bold text-white">{sessions.length}</div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Active</h3>
            <span className="text-2xl">🟢</span>
          </div>
          <div className="text-3xl font-bold text-green-400">{activeCount}</div>
          <div className="text-xs text-gray-500 mt-1">
            {sessions.length > 0 ? ((activeCount / sessions.length) * 100).toFixed(1) : 0}% of total
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Idle</h3>
            <span className="text-2xl">⚪</span>
          </div>
          <div className="text-3xl font-bold text-gray-400">{idleCount}</div>
          <div className="text-xs text-gray-500 mt-1">
            {sessions.length > 0 ? ((idleCount / sessions.length) * 100).toFixed(1) : 0}% of total
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Blocked</h3>
            <span className="text-2xl">🔒</span>
          </div>
          <div className="text-3xl font-bold text-red-400">{blockedCount}</div>
          <div className="text-xs text-gray-500 mt-1">
            {blockingCount} blocking
          </div>
        </motion.div>
      </div>

      {/* Blocking Alert */}
      {blockingCount > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4"
        >
          <div className="flex items-start justify-between">
            <div className="flex items-start space-x-3">
              <span className="text-2xl">⚠️</span>
              <div>
                <h3 className="text-lg font-semibold text-red-400">Blocking Detected</h3>
                <p className="text-sm text-red-300 mt-1">
                  {blockingCount} session(s) are blocking {blockedCount} other session(s).
                  This may impact performance.
                </p>
              </div>
            </div>
            <button
              onClick={() => setShowBlocking(!showBlocking)}
              className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors text-sm"
            >
              {showBlocking ? 'Hide' : 'View'} Blocking Tree
            </button>
          </div>
        </motion.div>
      )}

      {/* Blocking Tree */}
      {showBlocking && blockingData && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: 'auto' }}
          exit={{ opacity: 0, height: 0 }}
        >
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold text-white">Blocking Analysis</h2>
            <button
              onClick={refreshBlocking}
              disabled={blockingLoading}
              className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors disabled:opacity-50"
            >
              {blockingLoading ? 'Loading...' : 'Refresh'}
            </button>
          </div>
          <BlockingTree
            data={blockingData}
            onKillSession={handleKillSession}
          />
        </motion.div>
      )}

      {/* Error Display */}
      {error && (
        <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
          <p className="text-red-400">Error loading sessions: {error}</p>
        </div>
      )}

      {/* Sessions Table */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.4 }}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold text-white">All Sessions</h2>
        </div>

        {isLoading && sessions.length === 0 ? (
          <div className="bg-gray-800 rounded-lg p-8 text-center">
            <div className="text-gray-400">Loading sessions...</div>
          </div>
        ) : (
          <SessionTable
            sessions={sessions}
            onKillSession={handleKillSession}
          />
        )}
      </motion.div>

      {/* Help Text */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.6 }}
        className="bg-blue-900 bg-opacity-20 border border-blue-500 border-opacity-30 rounded-lg p-4"
      >
        <h3 className="text-sm font-semibold text-blue-300 mb-2">About Sessions</h3>
        <ul className="text-xs text-blue-200 space-y-1 list-disc list-inside">
          <li>
            <strong>Active:</strong> Session is currently executing a query
          </li>
          <li>
            <strong>Idle:</strong> Session is connected but not executing any query
          </li>
          <li>
            <strong>Idle in Transaction:</strong> Session is in a transaction but not currently executing
          </li>
          <li>
            <strong>Blocked:</strong> Session is waiting for a lock held by another session
          </li>
          <li>
            Use the Kill button to terminate problematic sessions. Use with caution as this will
            rollback any uncommitted transactions.
          </li>
        </ul>
      </motion.div>
    </div>
  );
}
