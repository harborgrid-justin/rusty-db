// ============================================================================
// Audit Logs Page
// View, filter, and export audit logs with statistics
// ============================================================================

import { useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import {
  ClipboardDocumentListIcon,
  CalendarIcon,
  ArrowDownTrayIcon,
  MagnifyingGlassIcon,
  ChartBarIcon,
  UserIcon,
  CheckCircleIcon,
  XCircleIcon,
} from '@heroicons/react/24/outline';
import {
  useAuditLogs,
  useAuditStatistics,
  useExportAuditLogs,
} from '../hooks/useSecurity';
import { AuditLogTable } from '../components/security/AuditLogTable';
import { SecurityEventTimeline } from '../components/security/SecurityEventTimeline';
import { LoadingScreen } from '../components/common/LoadingScreen';
import type { AuditEventType } from '../types';
import clsx from 'clsx';

// ============================================================================
// Audit Logs Component
// ============================================================================

export default function AuditLogs() {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(25);
  const [searchTerm, setSearchTerm] = useState('');
  const [eventTypeFilter, setEventTypeFilter] = useState<AuditEventType | 'all'>('all');
  const [statusFilter, setStatusFilter] = useState<'success' | 'failure' | 'all'>('all');
  const [usernameFilter, setUsernameFilter] = useState('');
  const [databaseFilter, setDatabaseFilter] = useState('');
  const [startTime, setStartTime] = useState<string>('');
  const [endTime, setEndTime] = useState<string>('');
  const [showStats, setShowStats] = useState(true);
  const [showTimeline, setShowTimeline] = useState(false);

  const filters = useMemo(
    () => ({
      page,
      pageSize,
      eventType: eventTypeFilter !== 'all' ? eventTypeFilter : undefined,
      status: statusFilter !== 'all' ? statusFilter : undefined,
      username: usernameFilter || undefined,
      database: databaseFilter || undefined,
      startTime: startTime || undefined,
      endTime: endTime || undefined,
    }),
    [
      page,
      pageSize,
      eventTypeFilter,
      statusFilter,
      usernameFilter,
      databaseFilter,
      startTime,
      endTime,
    ]
  );

  const { data: logsData, isLoading } = useAuditLogs(filters);
  const { data: stats } = useAuditStatistics(startTime || undefined, endTime || undefined);
  const exportLogs = useExportAuditLogs();

  const handleExport = async (format: 'csv' | 'json' | 'pdf') => {
    try {
      const result = await exportLogs.mutateAsync({
        filters,
        format,
      });
      if (result.data?.downloadUrl) {
        window.open(result.data.downloadUrl, '_blank');
      }
    } catch (error) {
      console.error('Failed to export logs:', error);
    }
  };

  const setTimeRange = (range: '1h' | '24h' | '7d' | '30d' | 'custom') => {
    const now = new Date();
    const start = new Date();

    switch (range) {
      case '1h':
        start.setHours(now.getHours() - 1);
        break;
      case '24h':
        start.setDate(now.getDate() - 1);
        break;
      case '7d':
        start.setDate(now.getDate() - 7);
        break;
      case '30d':
        start.setDate(now.getDate() - 30);
        break;
      case 'custom':
        return;
    }

    setStartTime(start.toISOString());
    setEndTime(now.toISOString());
  };

  if (isLoading && !logsData) {
    return <LoadingScreen />;
  }

  const successRate = stats
    ? (stats.eventsByStatus.success /
        (stats.eventsByStatus.success + stats.eventsByStatus.failure)) *
      100
    : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100 flex items-center gap-3">
            <ClipboardDocumentListIcon className="w-8 h-8 text-blue-500" />
            Audit Logs
          </h1>
          <p className="text-dark-400 mt-1">
            Monitor database activities and security events
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowStats(!showStats)}
            className={clsx('btn-secondary', showStats && 'bg-dark-700')}
          >
            <ChartBarIcon className="w-4 h-4" />
            Statistics
          </button>
          <button
            onClick={() => setShowTimeline(!showTimeline)}
            className={clsx('btn-secondary', showTimeline && 'bg-dark-700')}
          >
            <CalendarIcon className="w-4 h-4" />
            Timeline
          </button>
          <div className="relative group">
            <button className="btn-primary">
              <ArrowDownTrayIcon className="w-4 h-4" />
              Export
            </button>
            <div className="absolute right-0 mt-2 w-48 bg-dark-800 border border-dark-700 rounded-lg shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
              <button
                onClick={() => handleExport('csv')}
                className="dropdown-item w-full"
                disabled={exportLogs.isPending}
              >
                Export as CSV
              </button>
              <button
                onClick={() => handleExport('json')}
                className="dropdown-item w-full"
                disabled={exportLogs.isPending}
              >
                Export as JSON
              </button>
              <button
                onClick={() => handleExport('pdf')}
                className="dropdown-item w-full"
                disabled={exportLogs.isPending}
              >
                Export as PDF
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Statistics Cards */}
      {showStats && stats && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="grid grid-cols-1 md:grid-cols-4 gap-4"
        >
          <div className="card">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
                <ClipboardDocumentListIcon className="w-5 h-5 text-blue-400" />
              </div>
              <h3 className="text-sm font-medium text-dark-300">Total Events</h3>
            </div>
            <div className="flex items-baseline gap-2">
              <span className="text-3xl font-bold text-dark-100">{stats.totalEvents}</span>
            </div>
          </div>

          <div className="card">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-success-500/20 flex items-center justify-center">
                <CheckCircleIcon className="w-5 h-5 text-success-400" />
              </div>
              <h3 className="text-sm font-medium text-dark-300">Success Rate</h3>
            </div>
            <div className="flex items-baseline gap-2">
              <span className="text-3xl font-bold text-dark-100">
                {successRate.toFixed(1)}%
              </span>
            </div>
            <div className="mt-2 progress-bar">
              <div
                className="progress-fill bg-success-500"
                style={{ width: `${successRate}%` }}
              />
            </div>
          </div>

          <div className="card">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-danger-500/20 flex items-center justify-center">
                <XCircleIcon className="w-5 h-5 text-danger-400" />
              </div>
              <h3 className="text-sm font-medium text-dark-300">Failed Events</h3>
            </div>
            <div className="flex items-baseline gap-2">
              <span className="text-3xl font-bold text-dark-100">
                {stats.eventsByStatus.failure}
              </span>
              <span className="text-sm text-dark-400">
                ({((stats.eventsByStatus.failure / stats.totalEvents) * 100).toFixed(1)}%)
              </span>
            </div>
          </div>

          <div className="card">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-purple-500/20 flex items-center justify-center">
                <UserIcon className="w-5 h-5 text-purple-400" />
              </div>
              <h3 className="text-sm font-medium text-dark-300">Active Users</h3>
            </div>
            <div className="flex items-baseline gap-2">
              <span className="text-3xl font-bold text-dark-100">
                {stats.topUsers.length}
              </span>
              <span className="text-sm text-dark-400">users</span>
            </div>
          </div>
        </motion.div>
      )}

      {/* Event Type Breakdown */}
      {showStats && stats && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="card"
        >
          <h2 className="text-lg font-semibold text-dark-100 mb-4">Event Type Breakdown</h2>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {Object.entries(stats.eventsByType).map(([type, count]) => (
              <div
                key={type}
                className="p-3 rounded-lg bg-dark-700/50 hover:bg-dark-700 transition-colors cursor-pointer"
                onClick={() => setEventTypeFilter(type as AuditEventType)}
              >
                <div className="text-xs text-dark-400 mb-1">{type.toUpperCase()}</div>
                <div className="text-2xl font-bold text-dark-100">{count}</div>
                <div className="text-xs text-dark-400 mt-1">
                  {((count / stats.totalEvents) * 100).toFixed(1)}% of total
                </div>
              </div>
            ))}
          </div>
        </motion.div>
      )}

      {/* Top Users and Actions */}
      {showStats && stats && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <motion.div
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            className="card"
          >
            <h2 className="text-lg font-semibold text-dark-100 mb-4">Top Users</h2>
            <div className="space-y-2">
              {stats.topUsers.slice(0, 5).map((user, idx) => (
                <div key={user.username} className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-dark-400 w-6">#{idx + 1}</span>
                    <span className="font-medium text-dark-100">{user.username}</span>
                  </div>
                  <span className="text-sm text-dark-400">{user.eventCount} events</span>
                </div>
              ))}
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 10 }}
            animate={{ opacity: 1, x: 0 }}
            className="card"
          >
            <h2 className="text-lg font-semibold text-dark-100 mb-4">Top Actions</h2>
            <div className="space-y-2">
              {stats.topActions.slice(0, 5).map((action, idx) => (
                <div key={action.action} className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-dark-400 w-6">#{idx + 1}</span>
                    <span className="font-medium text-dark-100">{action.action}</span>
                  </div>
                  <span className="text-sm text-dark-400">{action.count} times</span>
                </div>
              ))}
            </div>
          </motion.div>
        </div>
      )}

      {/* Timeline */}
      {showTimeline && stats && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <SecurityEventTimeline timeline={stats.timeline} />
        </motion.div>
      )}

      {/* Filters */}
      <div className="card">
        <div className="space-y-4">
          {/* Search and Time Range */}
          <div className="flex flex-col lg:flex-row gap-4">
            <div className="flex-1 relative">
              <MagnifyingGlassIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-dark-400" />
              <input
                type="text"
                placeholder="Search logs..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="input-field pl-10 w-full"
              />
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => setTimeRange('1h')}
                className="btn-secondary text-sm"
              >
                1 Hour
              </button>
              <button
                onClick={() => setTimeRange('24h')}
                className="btn-secondary text-sm"
              >
                24 Hours
              </button>
              <button
                onClick={() => setTimeRange('7d')}
                className="btn-secondary text-sm"
              >
                7 Days
              </button>
              <button
                onClick={() => setTimeRange('30d')}
                className="btn-secondary text-sm"
              >
                30 Days
              </button>
            </div>
          </div>

          {/* Advanced Filters */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
            <div>
              <label className="label text-xs">Event Type</label>
              <select
                value={eventTypeFilter}
                onChange={(e) =>
                  setEventTypeFilter(e.target.value as AuditEventType | 'all')
                }
                className="input-field w-full"
              >
                <option value="all">All Types</option>
                <option value="authentication">Authentication</option>
                <option value="authorization">Authorization</option>
                <option value="ddl">DDL</option>
                <option value="dml">DML</option>
                <option value="dcl">DCL</option>
                <option value="configuration">Configuration</option>
                <option value="security">Security</option>
                <option value="system">System</option>
              </select>
            </div>

            <div>
              <label className="label text-xs">Status</label>
              <select
                value={statusFilter}
                onChange={(e) =>
                  setStatusFilter(e.target.value as 'success' | 'failure' | 'all')
                }
                className="input-field w-full"
              >
                <option value="all">All Status</option>
                <option value="success">Success</option>
                <option value="failure">Failure</option>
              </select>
            </div>

            <div>
              <label className="label text-xs">Username</label>
              <input
                type="text"
                placeholder="Filter by user..."
                value={usernameFilter}
                onChange={(e) => setUsernameFilter(e.target.value)}
                className="input-field w-full"
              />
            </div>

            <div>
              <label className="label text-xs">Database</label>
              <input
                type="text"
                placeholder="Filter by database..."
                value={databaseFilter}
                onChange={(e) => setDatabaseFilter(e.target.value)}
                className="input-field w-full"
              />
            </div>

            <div className="flex items-end">
              <button
                onClick={() => {
                  setEventTypeFilter('all');
                  setStatusFilter('all');
                  setUsernameFilter('');
                  setDatabaseFilter('');
                  setStartTime('');
                  setEndTime('');
                }}
                className="btn-secondary w-full"
              >
                Clear Filters
              </button>
            </div>
          </div>

          {/* Custom Date Range */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="label text-xs">Start Time</label>
              <input
                type="datetime-local"
                value={startTime ? new Date(startTime).toISOString().slice(0, 16) : ''}
                onChange={(e) => setStartTime(e.target.value ? new Date(e.target.value).toISOString() : '')}
                className="input-field w-full"
              />
            </div>
            <div>
              <label className="label text-xs">End Time</label>
              <input
                type="datetime-local"
                value={endTime ? new Date(endTime).toISOString().slice(0, 16) : ''}
                onChange={(e) => setEndTime(e.target.value ? new Date(e.target.value).toISOString() : '')}
                className="input-field w-full"
              />
            </div>
          </div>
        </div>
      </div>

      {/* Audit Logs Table */}
      <AuditLogTable
        logs={logsData?.data || []}
        isLoading={isLoading}
        pagination={{
          page: logsData?.page || 1,
          pageSize: logsData?.pageSize || 25,
          total: logsData?.total || 0,
          totalPages: logsData?.totalPages || 1,
          hasNext: logsData?.hasNext || false,
          hasPrevious: logsData?.hasPrevious || false,
        }}
        onPageChange={setPage}
        onPageSizeChange={setPageSize}
      />
    </div>
  );
}
