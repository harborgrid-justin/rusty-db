import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend, BarChart, Bar } from 'recharts';
import {
  ArrowPathIcon,
  LockClosedIcon,
  ExclamationTriangleIcon,
  ClockIcon,
  CheckCircleIcon,
  XCircleIcon,
  ChartBarIcon,
} from '@heroicons/react/24/outline';
import { Card } from '../components/common/Card';
import { Table } from '../components/common/Table';
import { Badge, StatusBadge } from '../components/common/Badge';
import { Button } from '../components/common/Button';
import { Modal } from '../components/common/Modal';
import { Select } from '../components/common/Select';
import { ConfirmDialog } from '../components/common/ConfirmDialog';
import type { UUID } from '../types';

// ============================================================================
// Types
// ============================================================================

interface TransactionStatus {
  activeTransactions: Transaction[];
  lockInfo: LockInfo;
  deadlockHistory: DeadlockEvent[];
  mvccStatus: MVCCStatus;
  walStats: WALStats;
  metrics: TransactionMetrics;
}

interface Transaction {
  id: UUID;
  state: 'active' | 'idle_in_transaction' | 'committing' | 'aborting';
  startTime: string;
  duration: number;
  isolationLevel: 'read_uncommitted' | 'read_committed' | 'repeatable_read' | 'serializable';
  database: string;
  user: string;
  query?: string;
  locksHeld: number;
  rowsAffected: number;
}

interface LockInfo {
  totalLocks: number;
  grantedLocks: number;
  waitingLocks: number;
  locksByType: { type: string; count: number }[];
  lockTree: LockTreeNode[];
}

interface LockTreeNode {
  transactionId: UUID;
  lockType: string;
  resourceId: string;
  blockedBy?: UUID[];
  blocking: UUID[];
  waitTime: number;
}

interface DeadlockEvent {
  id: UUID;
  timestamp: string;
  transactionsInvolved: UUID[];
  resolution: string;
  details: string;
}

interface MVCCStatus {
  oldestSnapshot: string;
  activeSnapshots: number;
  totalVersions: number;
  vacuumProgress: number;
  tupleVersions: { table: string; versions: number }[];
}

interface WALStats {
  currentLSN: string;
  writeLSN: string;
  flushLSN: string;
  walBuffers: number;
  walWrites: number;
  walSyncs: number;
  avgWriteTime: number;
}

interface TransactionMetrics {
  totalCommitted: number;
  totalAborted: number;
  totalActive: number;
  avgDuration: number;
  tps: number;
  history: { time: string; commits: number; aborts: number; active: number }[];
}

// ============================================================================
// Transactions Page Component
// ============================================================================

export default function Transactions() {
  const [status, setStatus] = useState<TransactionStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTab, setSelectedTab] = useState<'transactions' | 'locks' | 'deadlocks' | 'mvcc' | 'wal'>('transactions');
  const [selectedIsolationLevel, setSelectedIsolationLevel] = useState<string>('all');
  const [showRollbackModal, setShowRollbackModal] = useState(false);
  const [selectedTransaction, setSelectedTransaction] = useState<UUID | null>(null);
  const [refreshInterval, setRefreshInterval] = useState(10000);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, refreshInterval);
    return () => clearInterval(interval);
  }, [refreshInterval]);

  const loadData = async () => {
    try {
      setLoading(true);
      // Simulated API call - replace with actual service call
      // const data = await transactionService.getTransactionStatus();
      const data: TransactionStatus = {
        activeTransactions: [
          {
            id: 'tx-1',
            state: 'active',
            startTime: new Date(Date.now() - 30000).toISOString(),
            duration: 30,
            isolationLevel: 'read_committed',
            database: 'production',
            user: 'app_user',
            query: 'UPDATE orders SET status = ? WHERE id = ?',
            locksHeld: 3,
            rowsAffected: 1,
          },
          {
            id: 'tx-2',
            state: 'idle_in_transaction',
            startTime: new Date(Date.now() - 120000).toISOString(),
            duration: 120,
            isolationLevel: 'serializable',
            database: 'production',
            user: 'admin',
            query: 'SELECT * FROM users WHERE id = 123',
            locksHeld: 1,
            rowsAffected: 0,
          },
          {
            id: 'tx-3',
            state: 'active',
            startTime: new Date(Date.now() - 5000).toISOString(),
            duration: 5,
            isolationLevel: 'read_committed',
            database: 'analytics',
            user: 'report_user',
            query: 'INSERT INTO logs (event, data) VALUES (?, ?)',
            locksHeld: 2,
            rowsAffected: 1,
          },
        ],
        lockInfo: {
          totalLocks: 45,
          grantedLocks: 42,
          waitingLocks: 3,
          locksByType: [
            { type: 'Row Exclusive', count: 25 },
            { type: 'Share', count: 12 },
            { type: 'Exclusive', count: 5 },
            { type: 'Access Share', count: 3 },
          ],
          lockTree: [
            {
              transactionId: 'tx-1',
              lockType: 'Row Exclusive',
              resourceId: 'table:orders:row:123',
              blockedBy: [],
              blocking: ['tx-4'],
              waitTime: 0,
            },
            {
              transactionId: 'tx-4',
              lockType: 'Share',
              resourceId: 'table:orders:row:123',
              blockedBy: ['tx-1'],
              blocking: [],
              waitTime: 2500,
            },
          ],
        },
        deadlockHistory: [
          {
            id: 'dl-1',
            timestamp: new Date(Date.now() - 300000).toISOString(),
            transactionsInvolved: ['tx-10', 'tx-11'],
            resolution: 'tx-11 aborted',
            details: 'Circular lock dependency detected on table orders',
          },
          {
            id: 'dl-2',
            timestamp: new Date(Date.now() - 600000).toISOString(),
            transactionsInvolved: ['tx-8', 'tx-9'],
            resolution: 'tx-9 aborted',
            details: 'Deadlock on foreign key constraint update',
          },
        ],
        mvccStatus: {
          oldestSnapshot: new Date(Date.now() - 300000).toISOString(),
          activeSnapshots: 15,
          totalVersions: 12500,
          vacuumProgress: 65,
          tupleVersions: [
            { table: 'orders', versions: 5000 },
            { table: 'customers', versions: 3500 },
            { table: 'products', versions: 2500 },
            { table: 'logs', versions: 1500 },
          ],
        },
        walStats: {
          currentLSN: '0/1A2B3C4D',
          writeLSN: '0/1A2B3C40',
          flushLSN: '0/1A2B3C30',
          walBuffers: 2048,
          walWrites: 1250,
          walSyncs: 450,
          avgWriteTime: 1.2,
        },
        metrics: {
          totalCommitted: 45230,
          totalAborted: 325,
          totalActive: 3,
          avgDuration: 15.5,
          tps: 125.4,
          history: Array.from({ length: 20 }, (_, i) => ({
            time: new Date(Date.now() - (19 - i) * 60000).toLocaleTimeString(),
            commits: Math.floor(Math.random() * 150) + 100,
            aborts: Math.floor(Math.random() * 10),
            active: Math.floor(Math.random() * 5) + 1,
          })),
        },
      };
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load transaction data');
    } finally {
      setLoading(false);
    }
  };

  const handleRollback = async (transactionId: UUID) => {
    setSelectedTransaction(transactionId);
    setShowRollbackModal(true);
  };

  const confirmRollback = async () => {
    if (!selectedTransaction) return;
    try {
      // await transactionService.rollbackTransaction(selectedTransaction);
      console.log('Rolling back transaction:', selectedTransaction);
      setShowRollbackModal(false);
      setSelectedTransaction(null);
      await loadData();
    } catch (err) {
      console.error('Failed to rollback transaction:', err);
    }
  };

  const formatDuration = (seconds: number): string => {
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}m ${secs}s`;
  };

  if (loading && !status) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading transaction data...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
        <p className="text-red-400">Error: {error}</p>
      </div>
    );
  }

  if (!status) return null;

  const filteredTransactions =
    selectedIsolationLevel === 'all'
      ? status.activeTransactions
      : status.activeTransactions.filter((tx) => tx.isolationLevel === selectedIsolationLevel);

  const longRunningTx = status.activeTransactions.filter((tx) => tx.duration > 60).length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Transaction Monitoring</h1>
          <p className="text-gray-400 mt-1">Monitor active transactions, locks, and MVCC status</p>
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
          </select>
          <button
            onClick={loadData}
            disabled={loading}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors disabled:opacity-50 flex items-center space-x-2"
          >
            <ArrowPathIcon className="h-5 w-5" />
            <span>{loading ? 'Loading...' : 'Refresh'}</span>
          </button>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Active Transactions</h3>
            <ArrowPathIcon className="h-6 w-6 text-blue-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.metrics.totalActive}</div>
          <div className="text-xs text-gray-500 mt-1">
            {longRunningTx} long-running
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">TPS</h3>
            <ChartBarIcon className="h-6 w-6 text-green-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.metrics.tps.toFixed(1)}</div>
          <div className="text-xs text-gray-500 mt-1">
            transactions/sec
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Lock Waits</h3>
            <LockClosedIcon className="h-6 w-6 text-yellow-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.lockInfo.waitingLocks}</div>
          <div className="text-xs text-gray-500 mt-1">
            of {status.lockInfo.totalLocks} total
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Committed</h3>
            <CheckCircleIcon className="h-6 w-6 text-green-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.metrics.totalCommitted.toLocaleString()}</div>
          <div className="text-xs text-gray-500 mt-1">
            total commits
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Aborted</h3>
            <XCircleIcon className="h-6 w-6 text-red-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.metrics.totalAborted.toLocaleString()}</div>
          <div className="text-xs text-gray-500 mt-1">
            total rollbacks
          </div>
        </motion.div>
      </div>

      {/* Warning Banner */}
      {longRunningTx > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-yellow-900 bg-opacity-20 border border-yellow-500 rounded-lg p-4"
        >
          <div className="flex items-start space-x-3">
            <ExclamationTriangleIcon className="h-6 w-6 text-yellow-400 flex-shrink-0" />
            <div>
              <h3 className="text-lg font-semibold text-yellow-400">Long-Running Transactions Detected</h3>
              <p className="text-sm text-yellow-300 mt-1">
                {longRunningTx} transaction(s) have been running for more than 60 seconds.
                This may cause lock contention and affect performance.
              </p>
            </div>
          </div>
        </motion.div>
      )}

      {/* Tabs */}
      <div className="flex space-x-2 border-b border-gray-700">
        {(['transactions', 'locks', 'deadlocks', 'mvcc', 'wal'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setSelectedTab(tab)}
            className={`px-4 py-2 font-medium capitalize transition-colors ${
              selectedTab === tab
                ? 'text-blue-400 border-b-2 border-blue-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'mvcc' ? 'MVCC' : tab === 'wal' ? 'WAL' : tab}
          </button>
        ))}
      </div>

      {/* Transactions Tab */}
      {selectedTab === 'transactions' && (
        <>
          {/* Transaction Metrics Chart */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Transaction Activity</h3>
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={status.metrics.history}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <YAxis stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '0.5rem',
                  }}
                />
                <Legend />
                <Line type="monotone" dataKey="commits" stroke="#10b981" strokeWidth={2} name="Commits" dot={false} />
                <Line type="monotone" dataKey="aborts" stroke="#ef4444" strokeWidth={2} name="Aborts" dot={false} />
                <Line type="monotone" dataKey="active" stroke="#3b82f6" strokeWidth={2} name="Active" dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </motion.div>

          {/* Active Transactions Table */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-white">Active Transactions</h3>
              <Select
                value={selectedIsolationLevel}
                onChange={(e) => setSelectedIsolationLevel(e.target.value)}
                options={[
                  { value: 'all', label: 'All Isolation Levels' },
                  { value: 'read_uncommitted', label: 'Read Uncommitted' },
                  { value: 'read_committed', label: 'Read Committed' },
                  { value: 'repeatable_read', label: 'Repeatable Read' },
                  { value: 'serializable', label: 'Serializable' },
                ]}
              />
            </div>
            <Table
              columns={[
                { key: 'id', header: 'Transaction ID', sortable: true },
                {
                  key: 'state',
                  header: 'State',
                  render: (value) => (
                    <StatusBadge
                      status={value === 'active' ? 'active' : value === 'committing' ? 'success' : 'warning'}
                      label={value.replace('_', ' ')}
                    />
                  ),
                },
                {
                  key: 'duration',
                  header: 'Duration',
                  render: (value) => formatDuration(value),
                  sortable: true,
                },
                {
                  key: 'isolationLevel',
                  header: 'Isolation',
                  render: (value) => <Badge variant="info">{value.replace('_', ' ')}</Badge>,
                },
                { key: 'database', header: 'Database' },
                { key: 'user', header: 'User' },
                { key: 'locksHeld', header: 'Locks', sortable: true },
                { key: 'rowsAffected', header: 'Rows', sortable: true },
                {
                  key: 'query',
                  header: 'Query',
                  render: (value) => (
                    <div className="max-w-xs truncate text-sm font-mono" title={value}>
                      {value || '-'}
                    </div>
                  ),
                },
                {
                  key: 'actions',
                  header: 'Actions',
                  render: (_, row) => (
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => handleRollback(row.id)}
                    >
                      Rollback
                    </Button>
                  ),
                },
              ]}
              data={filteredTransactions}
              striped
              hoverable
              compact
            />
          </motion.div>
        </>
      )}

      {/* Locks Tab */}
      {selectedTab === 'locks' && (
        <>
          {/* Lock Distribution */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Lock Distribution</h3>
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={status.lockInfo.locksByType}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="type" stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <YAxis stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '0.5rem',
                  }}
                />
                <Bar dataKey="count" fill="#3b82f6" />
              </BarChart>
            </ResponsiveContainer>
          </motion.div>

          {/* Lock Tree */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Lock Visualization</h3>
            <Table
              columns={[
                { key: 'transactionId', header: 'Transaction', sortable: true },
                { key: 'lockType', header: 'Lock Type' },
                { key: 'resourceId', header: 'Resource' },
                {
                  key: 'blockedBy',
                  header: 'Blocked By',
                  render: (value) => (value && value.length > 0 ? value.join(', ') : '-'),
                },
                {
                  key: 'blocking',
                  header: 'Blocking',
                  render: (value) => (value && value.length > 0 ? value.join(', ') : '-'),
                },
                {
                  key: 'waitTime',
                  header: 'Wait Time',
                  render: (value) => (value > 0 ? `${(value / 1000).toFixed(1)}s` : '-'),
                  sortable: true,
                },
              ]}
              data={status.lockInfo.lockTree}
              striped
              hoverable
            />
          </motion.div>
        </>
      )}

      {/* Deadlocks Tab */}
      {selectedTab === 'deadlocks' && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">Deadlock History</h3>
          <Table
            columns={[
              {
                key: 'timestamp',
                header: 'Time',
                render: (value) => new Date(value).toLocaleString(),
                sortable: true,
              },
              {
                key: 'transactionsInvolved',
                header: 'Transactions',
                render: (value) => value.join(', '),
              },
              { key: 'resolution', header: 'Resolution' },
              { key: 'details', header: 'Details' },
            ]}
            data={status.deadlockHistory}
            striped
            hoverable
            emptyMessage="No deadlocks detected"
          />
        </motion.div>
      )}

      {/* MVCC Tab */}
      {selectedTab === 'mvcc' && (
        <>
          {/* MVCC Stats */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Active Snapshots</h3>
              <div className="text-3xl font-bold text-white">{status.mvccStatus.activeSnapshots}</div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Total Versions</h3>
              <div className="text-3xl font-bold text-white">{status.mvccStatus.totalVersions.toLocaleString()}</div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Vacuum Progress</h3>
              <div className="text-3xl font-bold text-white">{status.mvccStatus.vacuumProgress}%</div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.3 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Oldest Snapshot</h3>
              <div className="text-sm font-bold text-white">
                {formatDuration(Math.floor((Date.now() - new Date(status.mvccStatus.oldestSnapshot).getTime()) / 1000))}
              </div>
            </motion.div>
          </div>

          {/* Tuple Versions */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Tuple Versions by Table</h3>
            <Table
              columns={[
                { key: 'table', header: 'Table', sortable: true },
                {
                  key: 'versions',
                  header: 'Versions',
                  render: (value) => value.toLocaleString(),
                  sortable: true,
                },
              ]}
              data={status.mvccStatus.tupleVersions}
              striped
              hoverable
            />
          </motion.div>
        </>
      )}

      {/* WAL Tab */}
      {selectedTab === 'wal' && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">Write-Ahead Log Statistics</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Current LSN:</span>
                <span className="text-white font-medium font-mono">{status.walStats.currentLSN}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Write LSN:</span>
                <span className="text-white font-medium font-mono">{status.walStats.writeLSN}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Flush LSN:</span>
                <span className="text-white font-medium font-mono">{status.walStats.flushLSN}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">WAL Buffers:</span>
                <span className="text-white font-medium">{status.walStats.walBuffers}</span>
              </div>
            </div>
            <div className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">WAL Writes:</span>
                <span className="text-white font-medium">{status.walStats.walWrites.toLocaleString()}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">WAL Syncs:</span>
                <span className="text-white font-medium">{status.walStats.walSyncs.toLocaleString()}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Avg Write Time:</span>
                <span className="text-white font-medium">{status.walStats.avgWriteTime.toFixed(2)} ms</span>
              </div>
            </div>
          </div>
        </motion.div>
      )}

      {/* Rollback Confirmation Modal */}
      {showRollbackModal && (
        <ConfirmDialog
          isOpen={showRollbackModal}
          title="Confirm Rollback"
          message={`Are you sure you want to rollback transaction ${selectedTransaction}? This action cannot be undone.`}
          confirmLabel="Rollback"
          cancelLabel="Cancel"
          variant="danger"
          onConfirm={confirmRollback}
          onCancel={() => {
            setShowRollbackModal(false);
            setSelectedTransaction(null);
          }}
        />
      )}
    </div>
  );
}
