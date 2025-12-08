import { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { formatDistanceToNow } from 'date-fns';
import type { ActiveSession, UUID } from '../../types';

interface SessionTableProps {
  sessions: ActiveSession[];
  onKillSession?: (sessionId: UUID, force: boolean) => Promise<void>;
  className?: string;
}

export function SessionTable({ sessions, onKillSession, className = '' }: SessionTableProps) {
  const [filter, setFilter] = useState({
    state: '',
    user: '',
    database: '',
    search: '',
  });
  const [sortBy, setSortBy] = useState<keyof ActiveSession>('backendStart');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [confirmKill, setConfirmKill] = useState<{ id: UUID; force: boolean } | null>(null);
  const [isKilling, setIsKilling] = useState(false);

  const filteredSessions = useMemo(() => {
    return sessions.filter((session) => {
      if (filter.state && session.state !== filter.state) return false;
      if (filter.user && session.userId && !session.userId.toLowerCase().includes(filter.user.toLowerCase())) return false;
      if (filter.database && !session.database.toLowerCase().includes(filter.database.toLowerCase())) return false;
      if (filter.search) {
        const search = filter.search.toLowerCase();
        const matchesQuery = session.currentQuery?.toLowerCase().includes(search);
        const matchesUser = session.userId?.toLowerCase().includes(search);
        const matchesDb = session.database.toLowerCase().includes(search);
        const matchesIp = session.clientAddress.toLowerCase().includes(search);
        if (!matchesQuery && !matchesUser && !matchesDb && !matchesIp) return false;
      }
      return true;
    });
  }, [sessions, filter]);

  const sortedSessions = useMemo(() => {
    return [...filteredSessions].sort((a, b) => {
      const aVal = a[sortBy];
      const bVal = b[sortBy];

      if (aVal === undefined || aVal === null) return 1;
      if (bVal === undefined || bVal === null) return -1;

      let comparison = 0;
      if (typeof aVal === 'string' && typeof bVal === 'string') {
        comparison = aVal.localeCompare(bVal);
      } else if (typeof aVal === 'number' && typeof bVal === 'number') {
        comparison = aVal - bVal;
      } else {
        comparison = String(aVal).localeCompare(String(bVal));
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });
  }, [filteredSessions, sortBy, sortOrder]);

  const handleSort = (column: keyof ActiveSession) => {
    if (sortBy === column) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(column);
      setSortOrder('asc');
    }
  };

  const handleKillClick = (sessionId: UUID, force: boolean = false) => {
    setConfirmKill({ id: sessionId, force });
  };

  const handleConfirmKill = async () => {
    if (!confirmKill || !onKillSession) return;

    setIsKilling(true);
    try {
      await onKillSession(confirmKill.id, confirmKill.force);
      setConfirmKill(null);
    } catch (error) {
      alert(error instanceof Error ? error.message : 'Failed to kill session');
    } finally {
      setIsKilling(false);
    }
  };

  const getStateColor = (state: string) => {
    switch (state) {
      case 'active':
        return 'bg-green-500';
      case 'idle':
        return 'bg-gray-500';
      case 'idle_in_transaction':
        return 'bg-yellow-500';
      case 'idle_in_transaction_aborted':
        return 'bg-red-500';
      default:
        return 'bg-blue-500';
    }
  };

  const getStateBadge = (state: string) => {
    return (
      <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${getStateColor(state)} text-white`}>
        {state.replace(/_/g, ' ')}
      </span>
    );
  };

  const uniqueStates = [...new Set(sessions.map(s => s.state))];
  const uniqueUsers = [...new Set(sessions.map(s => s.userId).filter(Boolean))];
  const uniqueDatabases = [...new Set(sessions.map(s => s.database))];

  return (
    <div className={className}>
      {/* Filters */}
      <div className="mb-4 grid grid-cols-1 md:grid-cols-4 gap-3">
        <input
          type="text"
          placeholder="Search sessions..."
          value={filter.search}
          onChange={(e) => setFilter({ ...filter, search: e.target.value })}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />

        <select
          value={filter.state}
          onChange={(e) => setFilter({ ...filter, state: e.target.value })}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All States</option>
          {uniqueStates.map(state => (
            <option key={state} value={state}>{state}</option>
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
      </div>

      {/* Stats */}
      <div className="mb-4 flex items-center space-x-4 text-sm text-gray-400">
        <span>Total: {sortedSessions.length}</span>
        <span>Active: {sortedSessions.filter(s => s.state === 'active').length}</span>
        <span>Idle: {sortedSessions.filter(s => s.state === 'idle').length}</span>
      </div>

      {/* Table */}
      <div className="overflow-x-auto rounded-lg border border-gray-700">
        <table className="w-full text-sm">
          <thead className="bg-gray-800 border-b border-gray-700">
            <tr>
              <th
                onClick={() => handleSort('state')}
                className="px-4 py-3 text-left font-medium text-gray-300 cursor-pointer hover:bg-gray-700"
              >
                State {sortBy === 'state' && (sortOrder === 'asc' ? '↑' : '↓')}
              </th>
              <th
                onClick={() => handleSort('userId')}
                className="px-4 py-3 text-left font-medium text-gray-300 cursor-pointer hover:bg-gray-700"
              >
                User {sortBy === 'userId' && (sortOrder === 'asc' ? '↑' : '↓')}
              </th>
              <th
                onClick={() => handleSort('database')}
                className="px-4 py-3 text-left font-medium text-gray-300 cursor-pointer hover:bg-gray-700"
              >
                Database {sortBy === 'database' && (sortOrder === 'asc' ? '↑' : '↓')}
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-300">
                Client
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-300">
                Duration
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-300">
                Current Query
              </th>
              <th className="px-4 py-3 text-right font-medium text-gray-300">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-700 bg-gray-800">
            <AnimatePresence>
              {sortedSessions.map((session) => (
                <motion.tr
                  key={session.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="hover:bg-gray-700 transition-colors"
                >
                  <td className="px-4 py-3">
                    {getStateBadge(session.state)}
                  </td>
                  <td className="px-4 py-3 text-gray-300">
                    {session.userId || 'N/A'}
                  </td>
                  <td className="px-4 py-3 text-gray-300">
                    {session.database}
                  </td>
                  <td className="px-4 py-3 text-gray-400 text-xs">
                    {session.clientAddress}:{session.clientPort}
                  </td>
                  <td className="px-4 py-3 text-gray-400 text-xs">
                    {session.queryStart
                      ? formatDistanceToNow(new Date(session.queryStart), { addSuffix: false })
                      : formatDistanceToNow(new Date(session.backendStart), { addSuffix: false })}
                  </td>
                  <td className="px-4 py-3 max-w-md">
                    {session.currentQuery ? (
                      <code className="text-xs text-gray-400 truncate block">
                        {session.currentQuery.substring(0, 100)}
                        {session.currentQuery.length > 100 && '...'}
                      </code>
                    ) : (
                      <span className="text-gray-500 text-xs">No active query</span>
                    )}
                    {session.waitEvent && (
                      <span className="text-xs text-yellow-500 block mt-1">
                        Waiting: {session.waitEvent}
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-right">
                    <button
                      onClick={() => handleKillClick(session.id, false)}
                      className="px-2 py-1 text-xs font-medium text-red-400 hover:text-red-300 hover:bg-red-900 hover:bg-opacity-20 rounded transition-colors"
                    >
                      Kill
                    </button>
                  </td>
                </motion.tr>
              ))}
            </AnimatePresence>
          </tbody>
        </table>

        {sortedSessions.length === 0 && (
          <div className="p-8 text-center text-gray-500">
            No sessions found matching your filters
          </div>
        )}
      </div>

      {/* Confirm Kill Dialog */}
      {confirmKill && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4"
          >
            <h3 className="text-lg font-semibold text-white mb-4">
              Confirm Kill Session
            </h3>
            <p className="text-gray-300 mb-6">
              Are you sure you want to terminate this session? This action cannot be undone.
            </p>
            <div className="flex justify-end space-x-3">
              <button
                onClick={() => setConfirmKill(null)}
                disabled={isKilling}
                className="px-4 py-2 text-sm font-medium text-gray-300 bg-gray-700 rounded hover:bg-gray-600 disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmKill}
                disabled={isKilling}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded hover:bg-red-700 disabled:opacity-50"
              >
                {isKilling ? 'Killing...' : 'Kill Session'}
              </button>
            </div>
          </motion.div>
        </div>
      )}
    </div>
  );
}
