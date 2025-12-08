// ============================================================================
// Indexes Page
// Index management with usage statistics and recommendations
// ============================================================================

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  QueueListIcon,
  PlusIcon,
  ExclamationTriangleIcon,
  LightBulbIcon,
  ArrowPathIcon,
  FunnelIcon,
  MagnifyingGlassIcon,
} from '@heroicons/react/24/outline';
import {
  useIndexes,
  useUnusedIndexes,
  useIndexRecommendations,
  useCreateIndex,
  useDropIndex,
  useReindex,
} from '../hooks/useSchema';
import { IndexList } from '../components/schema/IndexList';
import type { Index } from '../types';
import type { CreateIndexRequest } from '../services/schemaService';
import { getErrorMessage } from '../services/api';
import clsx from 'clsx';

export default function Indexes() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showFilters, setShowFilters] = useState(false);
  const [tableFilter, setTableFilter] = useState('');
  const [showUnusedOnly, setShowUnusedOnly] = useState(false);

  const { data: indexes = [], isLoading, refetch } = useIndexes({
    tableName: tableFilter || undefined,
  });
  const { data: unusedIndexes = [] } = useUnusedIndexes(1);
  const { data: recommendations = [] } = useIndexRecommendations();

  const createIndexMutation = useCreateIndex();
  const dropIndexMutation = useDropIndex();
  const reindexMutation = useReindex();

  const filteredIndexes = showUnusedOnly
    ? indexes.filter((idx) => unusedIndexes.some((u) => u.name === idx.name))
    : indexes;

  const handleCreateIndex = async (definition: CreateIndexRequest) => {
    try {
      await createIndexMutation.mutateAsync(definition);
      setShowCreateModal(false);
      alert('Index created successfully');
    } catch (error) {
      alert(`Failed to create index: ${getErrorMessage(error)}`);
    }
  };

  const handleDropIndex = async (index: Index) => {
    if (!confirm(`Are you sure you want to drop index "${index.name}"?`)) {
      return;
    }

    const concurrent = confirm('Drop index concurrently (non-blocking)?');

    try {
      await dropIndexMutation.mutateAsync({
        indexName: index.name,
        concurrent,
      });
      alert('Index dropped successfully');
    } catch (error) {
      alert(`Failed to drop index: ${getErrorMessage(error)}`);
    }
  };

  const handleReindex = async (index: Index) => {
    const concurrent = confirm('Reindex concurrently (non-blocking)?');

    try {
      await reindexMutation.mutateAsync({
        target: index.name,
        concurrent,
      });
      alert('Reindex started successfully');
    } catch (error) {
      alert(`Failed to reindex: ${getErrorMessage(error)}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  const totalIndexSize = indexes.reduce((sum, idx) => sum + idx.size, 0);
  const unusedIndexSize = unusedIndexes.reduce((sum, idx) => sum + idx.size, 0);

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <div className="flex items-center gap-3 mb-2">
          <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
            <QueueListIcon className="w-6 h-6 text-rusty-400" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-dark-100">Indexes</h1>
            <p className="text-sm text-dark-400">
              Manage database indexes and optimize query performance
            </p>
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Indexes</p>
              <p className="text-2xl font-bold text-dark-100">{indexes.length}</p>
            </div>
            <div className="w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center">
              <QueueListIcon className="w-6 h-6 text-blue-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Unused Indexes</p>
              <p className="text-2xl font-bold text-warning-400">{unusedIndexes.length}</p>
            </div>
            <div className="w-12 h-12 bg-warning-500/20 rounded-lg flex items-center justify-center">
              <ExclamationTriangleIcon className="w-6 h-6 text-warning-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Size</p>
              <p className="text-2xl font-bold text-dark-100">{formatBytes(totalIndexSize)}</p>
            </div>
            <div className="w-12 h-12 bg-purple-500/20 rounded-lg flex items-center justify-center">
              <QueueListIcon className="w-6 h-6 text-purple-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Wasted Space</p>
              <p className="text-2xl font-bold text-danger-400">
                {formatBytes(unusedIndexSize)}
              </p>
            </div>
            <div className="w-12 h-12 bg-danger-500/20 rounded-lg flex items-center justify-center">
              <ExclamationTriangleIcon className="w-6 h-6 text-danger-400" />
            </div>
          </div>
        </div>
      </div>

      {/* Recommendations */}
      {recommendations.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4"
        >
          <div className="flex items-start gap-3">
            <LightBulbIcon className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <h3 className="text-sm font-medium text-blue-400 mb-2">
                Index Recommendations
              </h3>
              <div className="space-y-2">
                {recommendations.slice(0, 3).map((rec, index) => (
                  <div
                    key={index}
                    className="bg-dark-800/50 rounded p-3 text-sm"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1">
                        <p className="text-dark-200 font-medium mb-1">
                          Create index on {rec.tableName} ({rec.columns.join(', ')})
                        </p>
                        <p className="text-dark-400 text-xs mb-2">{rec.reason}</p>
                        <div className="flex items-center gap-2 text-xs text-dark-500">
                          <span>Estimated improvement: {rec.estimatedImprovement}%</span>
                          <span>•</span>
                          <span>{rec.queryPatterns.length} query patterns affected</span>
                        </div>
                      </div>
                      <button className="btn-sm btn-primary">Create</button>
                    </div>
                  </div>
                ))}
              </div>
              {recommendations.length > 3 && (
                <button className="text-sm text-blue-400 hover:text-blue-300 mt-3">
                  View all {recommendations.length} recommendations →
                </button>
              )}
            </div>
          </div>
        </motion.div>
      )}

      {/* Actions Bar */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowFilters(!showFilters)}
            className={clsx(
              'btn-secondary flex items-center gap-2',
              showFilters && 'bg-rusty-500/20 border-rusty-500/30'
            )}
          >
            <FunnelIcon className="w-4 h-4" />
            Filters
          </button>

          <button onClick={() => refetch()} className="btn-secondary flex items-center gap-2">
            <ArrowPathIcon className="w-4 h-4" />
            Refresh
          </button>

          <label className="flex items-center gap-2 px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg text-sm text-dark-300 cursor-pointer hover:bg-dark-700 transition-colors">
            <input
              type="checkbox"
              checked={showUnusedOnly}
              onChange={(e) => setShowUnusedOnly(e.target.checked)}
              className="rounded border-dark-600 bg-dark-800 text-rusty-500"
            />
            Show Unused Only
          </label>
        </div>

        <button
          onClick={() => setShowCreateModal(true)}
          className="btn-primary flex items-center gap-2"
        >
          <PlusIcon className="w-4 h-4" />
          Create Index
        </button>
      </div>

      {/* Filters Panel */}
      <AnimatePresence>
        {showFilters && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="card"
          >
            <div className="flex items-center gap-2">
              <MagnifyingGlassIcon className="w-5 h-5 text-dark-400" />
              <input
                type="text"
                value={tableFilter}
                onChange={(e) => setTableFilter(e.target.value)}
                className="input flex-1"
                placeholder="Filter by table name..."
              />
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Index List */}
      <div className="card">
        <IndexList
          indexes={filteredIndexes}
          onIndexDrop={handleDropIndex}
          onIndexReindex={handleReindex}
          isLoading={isLoading}
          showTable={true}
        />
      </div>

      {/* Create Index Modal */}
      <AnimatePresence>
        {showCreateModal && (
          <CreateIndexModal
            onClose={() => setShowCreateModal(false)}
            onCreate={handleCreateIndex}
            isCreating={createIndexMutation.isPending}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

// Create Index Modal Component
function CreateIndexModal({
  onClose,
  onCreate,
  isCreating,
}: {
  onClose: () => void;
  onCreate: (definition: CreateIndexRequest) => void;
  isCreating: boolean;
}) {
  const [tableName, setTableName] = useState('');
  const [indexName, setIndexName] = useState('');
  const [columns, setColumns] = useState<string[]>(['']);
  const [indexType, setIndexType] = useState<CreateIndexRequest['type']>('btree');
  const [unique, setUnique] = useState(false);
  const [concurrent, setConcurrent] = useState(true);
  const [whereClause, setWhereClause] = useState('');

  const handleAddColumn = () => {
    setColumns([...columns, '']);
  };

  const handleColumnChange = (index: number, value: string) => {
    const newColumns = [...columns];
    newColumns[index] = value;
    setColumns(newColumns);
  };

  const handleRemoveColumn = (index: number) => {
    setColumns(columns.filter((_, i) => i !== index));
  };

  const handleSubmit = () => {
    const filteredColumns = columns.filter((col) => col.trim());

    if (!tableName.trim()) {
      alert('Table name is required');
      return;
    }

    if (filteredColumns.length === 0) {
      alert('At least one column is required');
      return;
    }

    const definition: CreateIndexRequest = {
      tableName,
      name: indexName || undefined,
      columns: filteredColumns,
      type: indexType,
      unique,
      concurrent,
      where: whereClause || undefined,
    };

    onCreate(definition);
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.95, opacity: 0 }}
        onClick={(e) => e.stopPropagation()}
        className="card max-w-2xl w-full max-h-[90vh] overflow-y-auto"
      >
        <div className="border-b border-dark-700 px-6 py-4">
          <h2 className="text-xl font-semibold text-dark-100">Create Index</h2>
        </div>

        <div className="px-6 py-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Table Name *
              </label>
              <input
                type="text"
                value={tableName}
                onChange={(e) => setTableName(e.target.value)}
                className="input"
                placeholder="users"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Index Name
              </label>
              <input
                type="text"
                value={indexName}
                onChange={(e) => setIndexName(e.target.value)}
                className="input"
                placeholder="Auto-generated"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-dark-300 mb-2">
              Columns *
            </label>
            <div className="space-y-2">
              {columns.map((column, index) => (
                <div key={index} className="flex items-center gap-2">
                  <input
                    type="text"
                    value={column}
                    onChange={(e) => handleColumnChange(index, e.target.value)}
                    className="input flex-1"
                    placeholder="column_name"
                  />
                  {columns.length > 1 && (
                    <button
                      onClick={() => handleRemoveColumn(index)}
                      className="btn-secondary"
                    >
                      Remove
                    </button>
                  )}
                </div>
              ))}
            </div>
            <button onClick={handleAddColumn} className="btn-secondary btn-sm mt-2">
              Add Column
            </button>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Index Type
              </label>
              <select
                value={indexType}
                onChange={(e) => setIndexType(e.target.value as CreateIndexRequest['type'])}
                className="input"
              >
                <option value="btree">B-Tree</option>
                <option value="hash">Hash</option>
                <option value="gist">GiST</option>
                <option value="gin">GIN</option>
                <option value="brin">BRIN</option>
                <option value="spgist">SP-GiST</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Options
              </label>
              <div className="space-y-2">
                <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={unique}
                    onChange={(e) => setUnique(e.target.checked)}
                    className="rounded border-dark-600 bg-dark-800 text-rusty-500"
                  />
                  Unique
                </label>
                <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={concurrent}
                    onChange={(e) => setConcurrent(e.target.checked)}
                    className="rounded border-dark-600 bg-dark-800 text-rusty-500"
                  />
                  Create Concurrently
                </label>
              </div>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-dark-300 mb-2">
              WHERE Clause (Partial Index)
            </label>
            <input
              type="text"
              value={whereClause}
              onChange={(e) => setWhereClause(e.target.value)}
              className="input"
              placeholder="status = 'active'"
            />
          </div>
        </div>

        <div className="border-t border-dark-700 px-6 py-4 flex items-center justify-end gap-2">
          <button onClick={onClose} className="btn-secondary" disabled={isCreating}>
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            className="btn-primary"
            disabled={isCreating}
          >
            {isCreating ? 'Creating...' : 'Create Index'}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
