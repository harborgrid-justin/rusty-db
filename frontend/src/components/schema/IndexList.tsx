// ============================================================================
// Index List Component
// Displays indexes with usage statistics
// ============================================================================

import { motion } from 'framer-motion';
import {
  QueueListIcon,
  TrashIcon,
  ArrowPathIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon,
} from '@heroicons/react/24/outline';
import type { Index } from '../../types';
import clsx from 'clsx';

interface IndexListProps {
  indexes: Index[];
  onIndexDrop?: (index: Index) => void;
  onIndexReindex?: (index: Index) => void;
  isLoading?: boolean;
  showTable?: boolean;
}

export function IndexList({
  indexes,
  onIndexDrop,
  onIndexReindex,
  isLoading = false,
  showTable = true,
}: IndexListProps) {
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  const formatNumber = (num: number): string => {
    return new Intl.NumberFormat().format(num);
  };

  const formatDate = (date?: string): string => {
    if (!date) return 'Never';
    return new Date(date).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const isUnused = (index: Index): boolean => {
    return !index.usage.lastUsed || index.usage.scans === 0;
  };

  const getIndexTypeColor = (type: string): string => {
    const colors: Record<string, string> = {
      btree: 'text-blue-400',
      hash: 'text-green-400',
      gist: 'text-purple-400',
      gin: 'text-yellow-400',
      brin: 'text-orange-400',
      spgist: 'text-pink-400',
    };
    return colors[type] || 'text-dark-400';
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-32">
        <div className="flex items-center gap-3 text-dark-400">
          <ArrowPathIcon className="w-5 h-5 animate-spin" />
          <span>Loading indexes...</span>
        </div>
      </div>
    );
  }

  if (indexes.length === 0) {
    return (
      <div className="card text-center py-8">
        <QueueListIcon className="w-12 h-12 text-dark-400 mx-auto mb-3" />
        <h3 className="text-sm font-medium text-dark-200 mb-1">No Indexes Found</h3>
        <p className="text-sm text-dark-400">Create an index to improve query performance</p>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b border-dark-700">
            <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Name</th>
            {showTable && (
              <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                Table
              </th>
            )}
            <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
              Columns
            </th>
            <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Type</th>
            <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">Size</th>
            <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">
              Scans
            </th>
            <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
              Last Used
            </th>
            <th className="text-center py-3 px-4 text-sm font-medium text-dark-400">
              Status
            </th>
            <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          {indexes.map((index, i) => {
            const unused = isUnused(index);
            return (
              <motion.tr
                key={index.name}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.02 }}
                className={clsx(
                  'border-b border-dark-700/50 hover:bg-dark-800/50 transition-colors',
                  unused && 'bg-warning-500/5'
                )}
              >
                <td className="py-3 px-4">
                  <div className="flex items-center gap-2">
                    <QueueListIcon className="w-4 h-4 text-dark-400" />
                    <div>
                      <div className="text-dark-200 font-medium">{index.name}</div>
                      {index.isUnique && (
                        <span className="text-xs text-rusty-400">UNIQUE</span>
                      )}
                      {index.isPrimary && (
                        <span className="text-xs text-blue-400">PRIMARY</span>
                      )}
                    </div>
                  </div>
                </td>
                {showTable && (
                  <td className="py-3 px-4 text-dark-400 text-sm">
                    {/* We don't have table name in Index type, would need to add it */}
                    -
                  </td>
                )}
                <td className="py-3 px-4">
                  <div className="flex flex-wrap gap-1">
                    {index.columns.map((col) => (
                      <span
                        key={col}
                        className="px-2 py-0.5 bg-dark-700 rounded text-xs text-dark-300"
                      >
                        {col}
                      </span>
                    ))}
                  </div>
                </td>
                <td className="py-3 px-4">
                  <span className={clsx('text-sm font-medium', getIndexTypeColor(index.type))}>
                    {index.type.toUpperCase()}
                  </span>
                </td>
                <td className="py-3 px-4 text-right text-dark-200 text-sm">
                  {formatBytes(index.size)}
                </td>
                <td className="py-3 px-4 text-right text-dark-200 text-sm">
                  {formatNumber(index.usage.scans)}
                </td>
                <td className="py-3 px-4 text-dark-400 text-sm">
                  {formatDate(index.usage.lastUsed)}
                </td>
                <td className="py-3 px-4 text-center">
                  {unused ? (
                    <div
                      className="inline-flex items-center gap-1 px-2 py-1 bg-warning-500/10 border border-warning-500/20 rounded text-xs text-warning-400"
                      title="Index has not been used"
                    >
                      <ExclamationTriangleIcon className="w-3 h-3" />
                      Unused
                    </div>
                  ) : (
                    <div
                      className="inline-flex items-center gap-1 px-2 py-1 bg-success-500/10 border border-success-500/20 rounded text-xs text-success-400"
                      title="Index is being used"
                    >
                      <CheckCircleIcon className="w-3 h-3" />
                      Active
                    </div>
                  )}
                </td>
                <td className="py-3 px-4">
                  <div className="flex items-center justify-end gap-1">
                    {onIndexReindex && (
                      <button
                        onClick={() => onIndexReindex(index)}
                        className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
                        title="Reindex"
                      >
                        <ArrowPathIcon className="w-4 h-4" />
                      </button>
                    )}
                    {onIndexDrop && !index.isPrimary && (
                      <button
                        onClick={() => onIndexDrop(index)}
                        className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-danger-400 transition-colors"
                        title="Drop Index"
                        disabled={index.isPrimary}
                      >
                        <TrashIcon className="w-4 h-4" />
                      </button>
                    )}
                  </div>
                </td>
              </motion.tr>
            );
          })}
        </tbody>
      </table>

      {/* Summary */}
      <div className="mt-4 flex items-center justify-between text-sm text-dark-400">
        <span>{indexes.length} indexes</span>
        <span>
          Total size: {formatBytes(indexes.reduce((sum, idx) => sum + idx.size, 0))}
        </span>
      </div>

      {/* Unused Indexes Warning */}
      {indexes.some(isUnused) && (
        <div className="mt-4 bg-warning-500/10 border border-warning-500/20 rounded-lg p-4">
          <div className="flex items-start gap-3">
            <ExclamationTriangleIcon className="w-5 h-5 text-warning-400 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="text-sm font-medium text-warning-400 mb-1">
                Unused Indexes Detected
              </h4>
              <p className="text-sm text-dark-400">
                {indexes.filter(isUnused).length} index
                {indexes.filter(isUnused).length !== 1 ? 'es have' : ' has'} not been used.
                Consider dropping unused indexes to reduce storage and improve write
                performance.
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
