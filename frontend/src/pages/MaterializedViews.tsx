// ============================================================================
// Materialized Views Page
// View and materialized view management
// ============================================================================

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  RectangleGroupIcon,
  PlusIcon,
  ArrowPathIcon,
  TrashIcon,
  EyeIcon,
  ClockIcon,
  CircleStackIcon,
  CodeBracketIcon,
} from '@heroicons/react/24/outline';
import {
  useViews,
  useCreateView,
  useDropView,
  useRefreshMaterializedView,
  useViewDependencies,
} from '../hooks/useSchema';
import type { View } from '../types';
import type { CreateViewRequest } from '../services/schemaService';
import { getErrorMessage } from '../services/api';
import clsx from 'clsx';

export default function MaterializedViews() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [selectedView, setSelectedView] = useState<View | null>(null);
  const [showMaterializedOnly, setShowMaterializedOnly] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');

  const {
    data: views = [],
    isLoading,
    refetch,
  } = useViews({
    materializedOnly: showMaterializedOnly || undefined,
    search: searchTerm || undefined,
  });

  const createViewMutation = useCreateView();
  const dropViewMutation = useDropView();
  const refreshMutation = useRefreshMaterializedView();

  const materializedViews = views.filter((v) => v.isMaterialized);
  const regularViews = views.filter((v) => !v.isMaterialized);

  const handleCreateView = async (definition: CreateViewRequest) => {
    try {
      await createViewMutation.mutateAsync(definition);
      setShowCreateModal(false);
      alert('View created successfully');
    } catch (error) {
      alert(`Failed to create view: ${getErrorMessage(error)}`);
    }
  };

  const handleDropView = async (view: View) => {
    if (!confirm(`Are you sure you want to drop view "${view.name}"?`)) {
      return;
    }

    const cascade = confirm('Drop dependent objects as well (CASCADE)?');

    try {
      await dropViewMutation.mutateAsync({
        viewName: view.name,
        schema: view.schema,
        cascade,
        materialized: view.isMaterialized,
      });
      alert('View dropped successfully');
    } catch (error) {
      alert(`Failed to drop view: ${getErrorMessage(error)}`);
    }
  };

  const handleRefreshView = async (view: View) => {
    if (!view.isMaterialized) return;

    const concurrent = confirm('Refresh concurrently (non-blocking)?');

    try {
      await refreshMutation.mutateAsync({
        viewName: view.name,
        schema: view.schema,
        options: { concurrent },
      });
      alert('Materialized view refreshed successfully');
    } catch (error) {
      alert(`Failed to refresh view: ${getErrorMessage(error)}`);
    }
  };

  const formatDate = (date: string): string => {
    return new Date(date).toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <div className="flex items-center gap-3 mb-2">
          <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
            <RectangleGroupIcon className="w-6 h-6 text-rusty-400" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-dark-100">Views & Materialized Views</h1>
            <p className="text-sm text-dark-400">
              Manage database views and materialized views
            </p>
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Views</p>
              <p className="text-2xl font-bold text-dark-100">{views.length}</p>
            </div>
            <div className="w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center">
              <RectangleGroupIcon className="w-6 h-6 text-blue-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Regular Views</p>
              <p className="text-2xl font-bold text-dark-100">{regularViews.length}</p>
            </div>
            <div className="w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center">
              <EyeIcon className="w-6 h-6 text-green-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Materialized Views</p>
              <p className="text-2xl font-bold text-dark-100">{materializedViews.length}</p>
            </div>
            <div className="w-12 h-12 bg-purple-500/20 rounded-lg flex items-center justify-center">
              <CircleStackIcon className="w-6 h-6 text-purple-400" />
            </div>
          </div>
        </div>
      </div>

      {/* Actions Bar */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2">
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="input"
            placeholder="Search views..."
          />

          <label className="flex items-center gap-2 px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg text-sm text-dark-300 cursor-pointer hover:bg-dark-700 transition-colors">
            <input
              type="checkbox"
              checked={showMaterializedOnly}
              onChange={(e) => setShowMaterializedOnly(e.target.checked)}
              className="rounded border-dark-600 bg-dark-800 text-rusty-500"
            />
            Materialized Only
          </label>

          <button onClick={() => refetch()} className="btn-secondary flex items-center gap-2">
            <ArrowPathIcon className="w-4 h-4" />
            Refresh
          </button>
        </div>

        <button
          onClick={() => setShowCreateModal(true)}
          className="btn-primary flex items-center gap-2"
        >
          <PlusIcon className="w-4 h-4" />
          Create View
        </button>
      </div>

      {/* Views List */}
      <div className="card">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="flex items-center gap-3 text-dark-400">
              <div className="w-5 h-5 border-2 border-dark-400 border-t-rusty-500 rounded-full animate-spin" />
              <span>Loading views...</span>
            </div>
          </div>
        ) : views.length === 0 ? (
          <div className="text-center py-12">
            <RectangleGroupIcon className="w-16 h-16 text-dark-400 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-dark-200 mb-2">No Views Found</h3>
            <p className="text-dark-400 mb-6">Create your first view to get started</p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-dark-700">
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Name
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Type
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Schema
                  </th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">
                    Columns
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Last Refreshed
                  </th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">
                    Size
                  </th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody>
                {views.map((view, index) => (
                  <motion.tr
                    key={view.name}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: index * 0.02 }}
                    className="border-b border-dark-700/50 hover:bg-dark-800/50 transition-colors"
                  >
                    <td className="py-3 px-4">
                      <button
                        onClick={() => setSelectedView(view)}
                        className="flex items-center gap-2 text-dark-200 hover:text-rusty-400 transition-colors"
                      >
                        <RectangleGroupIcon className="w-4 h-4" />
                        <span className="font-medium">{view.name}</span>
                      </button>
                    </td>
                    <td className="py-3 px-4">
                      {view.isMaterialized ? (
                        <span className="px-2 py-1 bg-purple-500/20 border border-purple-500/30 rounded text-xs text-purple-400">
                          Materialized
                        </span>
                      ) : (
                        <span className="px-2 py-1 bg-blue-500/20 border border-blue-500/30 rounded text-xs text-blue-400">
                          Regular
                        </span>
                      )}
                    </td>
                    <td className="py-3 px-4 text-dark-400 text-sm">{view.schema}</td>
                    <td className="py-3 px-4 text-right text-dark-200 text-sm">
                      {view.columns.length}
                    </td>
                    <td className="py-3 px-4 text-dark-400 text-sm">
                      {view.isMaterialized && 'lastRefreshed' in view && view.lastRefreshed
                        ? formatDate(view.lastRefreshed)
                        : '-'}
                    </td>
                    <td className="py-3 px-4 text-right text-dark-200 text-sm">
                      {view.isMaterialized && 'size' in view
                        ? formatBytes(view.size)
                        : '-'}
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex items-center justify-end gap-1">
                        <button
                          onClick={() => setSelectedView(view)}
                          className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
                          title="View Definition"
                        >
                          <CodeBracketIcon className="w-4 h-4" />
                        </button>
                        {view.isMaterialized && (
                          <button
                            onClick={() => handleRefreshView(view)}
                            className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
                            title="Refresh View"
                          >
                            <ArrowPathIcon className="w-4 h-4" />
                          </button>
                        )}
                        <button
                          onClick={() => handleDropView(view)}
                          className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-danger-400 transition-colors"
                          title="Drop View"
                        >
                          <TrashIcon className="w-4 h-4" />
                        </button>
                      </div>
                    </td>
                  </motion.tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* View Details Modal */}
      <AnimatePresence>
        {selectedView && (
          <ViewDetailsModal view={selectedView} onClose={() => setSelectedView(null)} />
        )}
      </AnimatePresence>

      {/* Create View Modal */}
      <AnimatePresence>
        {showCreateModal && (
          <CreateViewModal
            onClose={() => setShowCreateModal(false)}
            onCreate={handleCreateView}
            isCreating={createViewMutation.isPending}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

// View Details Modal Component
function ViewDetailsModal({ view, onClose }: { view: View; onClose: () => void }) {
  const { data: dependencies } = useViewDependencies(view.name, view.schema);

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
        className="card max-w-4xl w-full max-h-[90vh] overflow-y-auto"
      >
        <div className="border-b border-dark-700 px-6 py-4">
          <h2 className="text-xl font-semibold text-dark-100">{view.name}</h2>
          <p className="text-sm text-dark-400 mt-1">
            {view.isMaterialized ? 'Materialized View' : 'View'} • {view.schema} schema
          </p>
        </div>

        <div className="px-6 py-6 space-y-6">
          {/* Definition */}
          <div>
            <h3 className="text-sm font-medium text-dark-300 mb-2">Definition</h3>
            <pre className="bg-dark-900 border border-dark-700 rounded-lg p-4 text-sm text-dark-200 overflow-x-auto">
              {view.definition}
            </pre>
          </div>

          {/* Columns */}
          <div>
            <h3 className="text-sm font-medium text-dark-300 mb-2">Columns</h3>
            <div className="grid grid-cols-2 gap-2">
              {view.columns.map((col) => (
                <div
                  key={col.name}
                  className="flex items-center gap-2 px-3 py-2 bg-dark-800 rounded"
                >
                  <span className="text-dark-200 text-sm">{col.name}</span>
                  <span className="text-dark-500 text-xs">{col.dataType}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Dependencies */}
          {dependencies && (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <h3 className="text-sm font-medium text-dark-300 mb-2">Dependencies</h3>
                {dependencies.dependencies.length === 0 ? (
                  <p className="text-sm text-dark-400 italic">None</p>
                ) : (
                  <ul className="space-y-1">
                    {dependencies.dependencies.map((dep) => (
                      <li key={dep} className="text-sm text-dark-300">
                        {dep}
                      </li>
                    ))}
                  </ul>
                )}
              </div>

              <div>
                <h3 className="text-sm font-medium text-dark-300 mb-2">Dependents</h3>
                {dependencies.dependents.length === 0 ? (
                  <p className="text-sm text-dark-400 italic">None</p>
                ) : (
                  <ul className="space-y-1">
                    {dependencies.dependents.map((dep) => (
                      <li key={dep} className="text-sm text-dark-300">
                        {dep}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="border-t border-dark-700 px-6 py-4 flex items-center justify-end">
          <button onClick={onClose} className="btn-secondary">
            Close
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}

// Create View Modal Component
function CreateViewModal({
  onClose,
  onCreate,
  isCreating,
}: {
  onClose: () => void;
  onCreate: (definition: CreateViewRequest) => void;
  isCreating: boolean;
}) {
  const [name, setName] = useState('');
  const [definition, setDefinition] = useState('SELECT ');
  const [materialized, setMaterialized] = useState(false);
  const [replace, setReplace] = useState(false);
  const [comment, setComment] = useState('');

  const handleSubmit = () => {
    if (!name.trim()) {
      alert('View name is required');
      return;
    }

    if (!definition.trim()) {
      alert('View definition is required');
      return;
    }

    const viewDef: CreateViewRequest = {
      name,
      definition,
      materialized,
      replace,
      comment: comment || undefined,
    };

    onCreate(viewDef);
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
        className="card max-w-3xl w-full max-h-[90vh] overflow-y-auto"
      >
        <div className="border-b border-dark-700 px-6 py-4">
          <h2 className="text-xl font-semibold text-dark-100">Create View</h2>
        </div>

        <div className="px-6 py-6 space-y-4">
          <div>
            <label className="block text-sm font-medium text-dark-300 mb-2">
              View Name *
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input"
              placeholder="my_view"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-dark-300 mb-2">
              Definition *
            </label>
            <textarea
              value={definition}
              onChange={(e) => setDefinition(e.target.value)}
              className="input font-mono text-sm"
              rows={10}
              placeholder="SELECT * FROM users WHERE active = true"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-dark-300 mb-2">Comment</label>
            <input
              type="text"
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              className="input"
              placeholder="Optional description"
            />
          </div>

          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
              <input
                type="checkbox"
                checked={materialized}
                onChange={(e) => setMaterialized(e.target.checked)}
                className="rounded border-dark-600 bg-dark-800 text-rusty-500"
              />
              Materialized View
            </label>

            <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
              <input
                type="checkbox"
                checked={replace}
                onChange={(e) => setReplace(e.target.checked)}
                className="rounded border-dark-600 bg-dark-800 text-rusty-500"
              />
              Replace if exists
            </label>
          </div>
        </div>

        <div className="border-t border-dark-700 px-6 py-4 flex items-center justify-end gap-2">
          <button onClick={onClose} className="btn-secondary" disabled={isCreating}>
            Cancel
          </button>
          <button onClick={handleSubmit} className="btn-primary" disabled={isCreating}>
            {isCreating ? 'Creating...' : 'Create View'}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
