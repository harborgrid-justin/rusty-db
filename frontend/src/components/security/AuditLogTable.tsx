// ============================================================================
// Audit Log Table Component
// Displays audit logs with filtering and SQL preview
// ============================================================================

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  CodeBracketIcon,
  CheckCircleIcon,
  XCircleIcon,
  UserIcon,
  CalendarIcon,
  CommandLineIcon,
  ShieldCheckIcon,
  LockClosedIcon,
  CogIcon,
  ServerIcon,
  TableCellsIcon,
} from '@heroicons/react/24/outline';
import type { AuditLog, AuditEventType } from '../../types';
import clsx from 'clsx';

// ============================================================================
// Component Props
// ============================================================================

interface AuditLogTableProps {
  logs: AuditLog[];
  isLoading: boolean;
  pagination: {
    page: number;
    pageSize: number;
    total: number;
    totalPages: number;
    hasNext: boolean;
    hasPrevious: boolean;
  };
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
}

// ============================================================================
// Audit Log Table Component
// ============================================================================

export function AuditLogTable({
  logs,
  isLoading,
  pagination,
  onPageChange,
  onPageSizeChange,
}: AuditLogTableProps) {
  const [selectedLog, setSelectedLog] = useState<AuditLog | null>(null);

  const getEventTypeIcon = (eventType: AuditEventType) => {
    switch (eventType) {
      case 'authentication':
        return UserIcon;
      case 'authorization':
        return ShieldCheckIcon;
      case 'ddl':
        return TableCellsIcon;
      case 'dml':
        return CommandLineIcon;
      case 'dcl':
        return LockClosedIcon;
      case 'configuration':
        return CogIcon;
      case 'security':
        return ShieldCheckIcon;
      case 'system':
        return ServerIcon;
      default:
        return CommandLineIcon;
    }
  };

  const getEventTypeColor = (eventType: AuditEventType) => {
    switch (eventType) {
      case 'authentication':
        return 'text-blue-400';
      case 'authorization':
        return 'text-purple-400';
      case 'ddl':
        return 'text-yellow-400';
      case 'dml':
        return 'text-green-400';
      case 'dcl':
        return 'text-red-400';
      case 'configuration':
        return 'text-orange-400';
      case 'security':
        return 'text-rusty-400';
      case 'system':
        return 'text-gray-400';
      default:
        return 'text-dark-400';
    }
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-dark-100">Audit Events</h2>
        <div className="flex items-center gap-3">
          <span className="text-sm text-dark-400">
            Showing {logs.length} of {pagination.total} events
          </span>
          <select
            value={pagination.pageSize}
            onChange={(e) => onPageSizeChange(Number(e.target.value))}
            className="input-field w-24 text-sm"
          >
            <option value="10">10</option>
            <option value="25">25</option>
            <option value="50">50</option>
            <option value="100">100</option>
          </select>
        </div>
      </div>

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="table">
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Event Type</th>
              <th>User</th>
              <th>Action</th>
              <th>Object</th>
              <th>Database</th>
              <th>Status</th>
              <th>SQL</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              <tr>
                <td colSpan={8} className="text-center py-8">
                  <div className="flex items-center justify-center gap-3">
                    <div className="w-5 h-5 border-2 border-rusty-500 border-t-transparent rounded-full animate-spin" />
                    <span className="text-dark-400">Loading audit logs...</span>
                  </div>
                </td>
              </tr>
            ) : logs.length === 0 ? (
              <tr>
                <td colSpan={8} className="text-center py-8">
                  <p className="text-dark-400">No audit logs found</p>
                </td>
              </tr>
            ) : (
              logs.map((log) => {
                const EventIcon = getEventTypeIcon(log.eventType);
                const eventColor = getEventTypeColor(log.eventType);

                return (
                  <tr key={log.id} className="hover:bg-dark-700/50">
                    <td className="text-sm">
                      <div className="flex items-center gap-2">
                        <CalendarIcon className="w-4 h-4 text-dark-400" />
                        {new Date(log.timestamp).toLocaleString()}
                      </div>
                    </td>
                    <td>
                      <div className="flex items-center gap-2">
                        <EventIcon className={clsx('w-4 h-4', eventColor)} />
                        <span className="text-sm capitalize">
                          {log.eventType.replace('_', ' ')}
                        </span>
                      </div>
                    </td>
                    <td className="text-sm">
                      <div>
                        <div className="font-medium text-dark-100">
                          {log.username || 'Unknown'}
                        </div>
                        {log.clientAddress && (
                          <div className="text-xs text-dark-400">{log.clientAddress}</div>
                        )}
                      </div>
                    </td>
                    <td className="text-sm font-medium">{log.action}</td>
                    <td className="text-sm">
                      {log.objectType && log.objectName ? (
                        <div className="font-mono">
                          <div className="text-xs text-dark-400">{log.objectType}</div>
                          <div className="text-dark-100">{log.objectName}</div>
                        </div>
                      ) : (
                        <span className="text-dark-500">-</span>
                      )}
                    </td>
                    <td className="text-sm font-mono">
                      {log.database || <span className="text-dark-500">-</span>}
                    </td>
                    <td>
                      <span
                        className={clsx(
                          'badge',
                          log.status === 'success' ? 'badge-success' : 'badge-danger'
                        )}
                      >
                        {log.status === 'success' ? (
                          <CheckCircleIcon className="w-3 h-3" />
                        ) : (
                          <XCircleIcon className="w-3 h-3" />
                        )}
                        {log.status}
                      </span>
                    </td>
                    <td>
                      {log.sqlText ? (
                        <button
                          onClick={() => setSelectedLog(log)}
                          className="btn-ghost text-xs"
                          title="View SQL"
                        >
                          <CodeBracketIcon className="w-4 h-4" />
                        </button>
                      ) : (
                        <span className="text-dark-500">-</span>
                      )}
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {pagination.totalPages > 1 && (
        <div className="flex items-center justify-between mt-4 pt-4 border-t border-dark-700">
          <div className="text-sm text-dark-400">
            Page {pagination.page} of {pagination.totalPages}
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => onPageChange(pagination.page - 1)}
              disabled={!pagination.hasPrevious}
              className="btn-secondary text-sm"
            >
              <ChevronLeftIcon className="w-4 h-4" />
              Previous
            </button>
            <div className="flex gap-1">
              {Array.from({ length: Math.min(5, pagination.totalPages) }, (_, i) => {
                let pageNum;
                if (pagination.totalPages <= 5) {
                  pageNum = i + 1;
                } else if (pagination.page <= 3) {
                  pageNum = i + 1;
                } else if (pagination.page >= pagination.totalPages - 2) {
                  pageNum = pagination.totalPages - 4 + i;
                } else {
                  pageNum = pagination.page - 2 + i;
                }

                return (
                  <button
                    key={pageNum}
                    onClick={() => onPageChange(pageNum)}
                    className={clsx(
                      'px-3 py-1.5 rounded-lg text-sm font-medium transition-colors',
                      pagination.page === pageNum
                        ? 'bg-rusty-500 text-white'
                        : 'bg-dark-700 text-dark-300 hover:bg-dark-600'
                    )}
                  >
                    {pageNum}
                  </button>
                );
              })}
            </div>
            <button
              onClick={() => onPageChange(pagination.page + 1)}
              disabled={!pagination.hasNext}
              className="btn-secondary text-sm"
            >
              Next
              <ChevronRightIcon className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* SQL Preview Modal */}
      {selectedLog && (
        <SQLPreviewModal log={selectedLog} onClose={() => setSelectedLog(null)} />
      )}
    </div>
  );
}

// ============================================================================
// SQL Preview Modal Component
// ============================================================================

interface SQLPreviewModalProps {
  log: AuditLog;
  onClose: () => void;
}

function SQLPreviewModal({ log, onClose }: SQLPreviewModalProps) {
  return (
    <div
      className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        onClick={(e) => e.stopPropagation()}
        className="card max-w-4xl w-full max-h-[80vh] overflow-y-auto"
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-4 pb-4 border-b border-dark-700">
          <h2 className="text-xl font-bold text-dark-100 flex items-center gap-3">
            <CodeBracketIcon className="w-6 h-6 text-rusty-500" />
            SQL Statement
          </h2>
          <button onClick={onClose} className="btn-ghost">
            <XCircleIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Event Details */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4 p-4 rounded-lg bg-dark-700/50">
          <div>
            <div className="text-xs text-dark-400 mb-1">Timestamp</div>
            <div className="text-sm text-dark-100">
              {new Date(log.timestamp).toLocaleString()}
            </div>
          </div>
          <div>
            <div className="text-xs text-dark-400 mb-1">User</div>
            <div className="text-sm text-dark-100">{log.username || 'Unknown'}</div>
          </div>
          <div>
            <div className="text-xs text-dark-400 mb-1">Database</div>
            <div className="text-sm text-dark-100 font-mono">
              {log.database || 'N/A'}
            </div>
          </div>
          <div>
            <div className="text-xs text-dark-400 mb-1">Status</div>
            <span
              className={clsx(
                'badge',
                log.status === 'success' ? 'badge-success' : 'badge-danger'
              )}
            >
              {log.status === 'success' ? (
                <CheckCircleIcon className="w-3 h-3" />
              ) : (
                <XCircleIcon className="w-3 h-3" />
              )}
              {log.status}
            </span>
          </div>
        </div>

        {/* SQL Text */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-dark-100">SQL Statement</h3>
            <button
              onClick={() => {
                navigator.clipboard.writeText(log.sqlText || '');
              }}
              className="btn-ghost text-xs"
            >
              Copy
            </button>
          </div>
          <pre className="p-4 rounded-lg bg-dark-900 border border-dark-700 overflow-x-auto">
            <code className="text-sm text-dark-100 font-mono">{log.sqlText}</code>
          </pre>
        </div>

        {/* Error Message */}
        {log.status === 'failure' && log.errorMessage && (
          <div className="mt-4 p-4 rounded-lg bg-danger-500/10 border border-danger-500/30">
            <h3 className="text-sm font-medium text-danger-400 mb-2">Error Message</h3>
            <p className="text-sm text-dark-100 font-mono">{log.errorMessage}</p>
          </div>
        )}

        {/* Metadata */}
        {log.metadata && Object.keys(log.metadata).length > 0 && (
          <div className="mt-4">
            <h3 className="text-sm font-medium text-dark-100 mb-2">Additional Metadata</h3>
            <pre className="p-4 rounded-lg bg-dark-900 border border-dark-700 overflow-x-auto">
              <code className="text-sm text-dark-100 font-mono">
                {JSON.stringify(log.metadata, null, 2)}
              </code>
            </pre>
          </div>
        )}

        {/* Actions */}
        <div className="mt-6 flex justify-end">
          <button onClick={onClose} className="btn-primary">
            Close
          </button>
        </div>
      </motion.div>
    </div>
  );
}
