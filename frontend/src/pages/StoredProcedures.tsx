// ============================================================================
// Stored Procedures Page
// Manage stored procedures and functions
// ============================================================================

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  CodeBracketIcon,
  PlusIcon,
  ArrowPathIcon,
  TrashIcon,
  PlayIcon,
} from '@heroicons/react/24/outline';
import {
  useProcedures,
  useCreateProcedure,
  useDropProcedure,
  useExecuteProcedure,
} from '../hooks/useSchema';
import type { StoredProcedure } from '../services/schemaService';
import type { CreateProcedureRequest } from '../services/schemaService';
import { getErrorMessage } from '../services/api';
import clsx from 'clsx';

export default function StoredProcedures() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [selectedProcedure, setSelectedProcedure] = useState<StoredProcedure | null>(null);
  const [showExecuteModal, setShowExecuteModal] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');

  const {
    data: procedures = [],
    isLoading,
    refetch,
  } = useProcedures({
    search: searchTerm || undefined,
  });

  const createProcedureMutation = useCreateProcedure();
  const dropProcedureMutation = useDropProcedure();

  const handleCreateProcedure = async (definition: CreateProcedureRequest) => {
    try {
      await createProcedureMutation.mutateAsync(definition);
      setShowCreateModal(false);
      alert('Procedure created successfully');
    } catch (error) {
      alert(`Failed to create procedure: ${getErrorMessage(error)}`);
    }
  };

  const handleDropProcedure = async (procedure: StoredProcedure) => {
    if (!confirm(`Are you sure you want to drop procedure "${procedure.name}"?`)) {
      return;
    }

    const cascade = confirm('Drop dependent objects as well (CASCADE)?');

    try {
      await dropProcedureMutation.mutateAsync({
        procedureName: procedure.name,
        schema: procedure.schema,
        cascade,
      });
      alert('Procedure dropped successfully');
    } catch (error) {
      alert(`Failed to drop procedure: ${getErrorMessage(error)}`);
    }
  };

  const handleExecuteProcedure = (procedure: StoredProcedure) => {
    setSelectedProcedure(procedure);
    setShowExecuteModal(true);
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

  const getVolatilityBadgeColor = (volatility: string) => {
    switch (volatility) {
      case 'immutable':
        return 'bg-green-500/20 border-green-500/30 text-green-400';
      case 'stable':
        return 'bg-blue-500/20 border-blue-500/30 text-blue-400';
      case 'volatile':
        return 'bg-orange-500/20 border-orange-500/30 text-orange-400';
      default:
        return 'bg-dark-700 text-dark-400';
    }
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <div className="flex items-center gap-3 mb-2">
          <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
            <CodeBracketIcon className="w-6 h-6 text-rusty-400" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-dark-100">Stored Procedures</h1>
            <p className="text-sm text-dark-400">
              Manage database stored procedures and functions
            </p>
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Procedures</p>
              <p className="text-2xl font-bold text-dark-100">{procedures.length}</p>
            </div>
            <div className="w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center">
              <CodeBracketIcon className="w-6 h-6 text-blue-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Languages</p>
              <p className="text-2xl font-bold text-dark-100">
                {new Set(procedures.map((p) => p.language)).size}
              </p>
            </div>
            <div className="w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center">
              <CodeBracketIcon className="w-6 h-6 text-green-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Security Definers</p>
              <p className="text-2xl font-bold text-dark-100">
                {procedures.filter((p) => p.securityDefiner).length}
              </p>
            </div>
            <div className="w-12 h-12 bg-purple-500/20 rounded-lg flex items-center justify-center">
              <CodeBracketIcon className="w-6 h-6 text-purple-400" />
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
            placeholder="Search procedures..."
          />

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
          Create Procedure
        </button>
      </div>

      {/* Procedures List */}
      <div className="card">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="flex items-center gap-3 text-dark-400">
              <div className="w-5 h-5 border-2 border-dark-400 border-t-rusty-500 rounded-full animate-spin" />
              <span>Loading procedures...</span>
            </div>
          </div>
        ) : procedures.length === 0 ? (
          <div className="text-center py-12">
            <CodeBracketIcon className="w-16 h-16 text-dark-400 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-dark-200 mb-2">
              No Procedures Found
            </h3>
            <p className="text-dark-400 mb-6">
              Create your first stored procedure to get started
            </p>
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
                    Language
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Return Type
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Parameters
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Volatility
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                    Modified
                  </th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-dark-400">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody>
                {procedures.map((proc, index) => (
                  <motion.tr
                    key={proc.name}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: index * 0.02 }}
                    className="border-b border-dark-700/50 hover:bg-dark-800/50 transition-colors"
                  >
                    <td className="py-3 px-4">
                      <button
                        onClick={() => setSelectedProcedure(proc)}
                        className="flex items-center gap-2 text-dark-200 hover:text-rusty-400 transition-colors"
                      >
                        <CodeBracketIcon className="w-4 h-4" />
                        <span className="font-medium">{proc.name}</span>
                      </button>
                    </td>
                    <td className="py-3 px-4">
                      <span className="px-2 py-1 bg-blue-500/20 border border-blue-500/30 rounded text-xs text-blue-400">
                        {proc.language.toUpperCase()}
                      </span>
                    </td>
                    <td className="py-3 px-4 text-dark-300 font-mono text-sm">
                      {proc.returnType}
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex flex-wrap gap-1">
                        {proc.parameters.length === 0 ? (
                          <span className="text-dark-500 text-sm">None</span>
                        ) : (
                          proc.parameters.slice(0, 3).map((param, i) => (
                            <span
                              key={i}
                              className="px-2 py-0.5 bg-dark-700 rounded text-xs text-dark-300"
                            >
                              {param.name}: {param.type}
                            </span>
                          ))
                        )}
                        {proc.parameters.length > 3 && (
                          <span className="text-xs text-dark-500">
                            +{proc.parameters.length - 3} more
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="py-3 px-4">
                      <span
                        className={clsx(
                          'px-2 py-1 border rounded text-xs',
                          getVolatilityBadgeColor(proc.volatility)
                        )}
                      >
                        {proc.volatility.toUpperCase()}
                      </span>
                    </td>
                    <td className="py-3 px-4 text-dark-400 text-sm">
                      {formatDate(proc.updatedAt)}
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex items-center justify-end gap-1">
                        <button
                          onClick={() => handleExecuteProcedure(proc)}
                          className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-success-400 transition-colors"
                          title="Execute"
                        >
                          <PlayIcon className="w-4 h-4" />
                        </button>
                        <button
                          onClick={() => setSelectedProcedure(proc)}
                          className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
                          title="View"
                        >
                          <CodeBracketIcon className="w-4 h-4" />
                        </button>
                        <button
                          onClick={() => handleDropProcedure(proc)}
                          className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-danger-400 transition-colors"
                          title="Drop"
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

      {/* Procedure Details Modal */}
      <AnimatePresence>
        {selectedProcedure && !showExecuteModal && (
          <ProcedureDetailsModal
            procedure={selectedProcedure}
            onClose={() => setSelectedProcedure(null)}
          />
        )}
      </AnimatePresence>

      {/* Execute Procedure Modal */}
      <AnimatePresence>
        {showExecuteModal && selectedProcedure && (
          <ExecuteProcedureModal
            procedure={selectedProcedure}
            onClose={() => {
              setShowExecuteModal(false);
              setSelectedProcedure(null);
            }}
          />
        )}
      </AnimatePresence>

      {/* Create Procedure Modal */}
      <AnimatePresence>
        {showCreateModal && (
          <CreateProcedureModal
            onClose={() => setShowCreateModal(false)}
            onCreate={handleCreateProcedure}
            isCreating={createProcedureMutation.isPending}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

// Procedure Details Modal Component
function ProcedureDetailsModal({
  procedure,
  onClose,
}: {
  procedure: StoredProcedure;
  onClose: () => void;
}) {
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
          <h2 className="text-xl font-semibold text-dark-100">{procedure.name}</h2>
          <p className="text-sm text-dark-400 mt-1">
            {procedure.language} procedure • {procedure.schema} schema
          </p>
        </div>

        <div className="px-6 py-6 space-y-6">
          {/* Info Grid */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <h4 className="text-sm font-medium text-dark-400 mb-1">Return Type</h4>
              <p className="text-dark-200 font-mono">{procedure.returnType}</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-dark-400 mb-1">Volatility</h4>
              <p className="text-dark-200">{procedure.volatility.toUpperCase()}</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-dark-400 mb-1">Strict</h4>
              <p className="text-dark-200">{procedure.isStrict ? 'Yes' : 'No'}</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-dark-400 mb-1">Security Definer</h4>
              <p className="text-dark-200">{procedure.securityDefiner ? 'Yes' : 'No'}</p>
            </div>
          </div>

          {/* Parameters */}
          <div>
            <h3 className="text-sm font-medium text-dark-300 mb-2">Parameters</h3>
            {procedure.parameters.length === 0 ? (
              <p className="text-sm text-dark-400 italic">No parameters</p>
            ) : (
              <div className="space-y-2">
                {procedure.parameters.map((param, index) => (
                  <div
                    key={index}
                    className="flex items-center gap-4 px-4 py-2 bg-dark-800 rounded"
                  >
                    <span className="px-2 py-0.5 bg-dark-700 rounded text-xs text-dark-400">
                      {param.mode.toUpperCase()}
                    </span>
                    <span className="text-dark-200 font-medium">{param.name}</span>
                    <span className="text-dark-400 font-mono text-sm">{param.type}</span>
                    {param.defaultValue && (
                      <span className="text-dark-500 text-sm ml-auto">
                        = {param.defaultValue}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Definition */}
          <div>
            <h3 className="text-sm font-medium text-dark-300 mb-2">Definition</h3>
            <pre className="bg-dark-900 border border-dark-700 rounded-lg p-4 text-sm text-dark-200 overflow-x-auto">
              {procedure.definition}
            </pre>
          </div>
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

// Execute Procedure Modal Component
function ExecuteProcedureModal({
  procedure,
  onClose,
}: {
  procedure: StoredProcedure;
  onClose: () => void;
}) {
  const [parameters, setParameters] = useState<Record<string, unknown>>(() => {
    const initial: Record<string, unknown> = {};
    procedure.parameters.forEach((param) => {
      initial[param.name] = param.defaultValue || '';
    });
    return initial;
  });
  const [result, setResult] = useState<unknown>(null);
  const [error, setError] = useState<string | null>(null);

  const executeMutation = useExecuteProcedure(procedure.name, procedure.schema);

  const handleExecute = async () => {
    setError(null);
    setResult(null);

    try {
      const response = await executeMutation.mutateAsync({ parameters });
      setResult(response.data);
    } catch (err) {
      setError(getErrorMessage(err));
    }
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
          <h2 className="text-xl font-semibold text-dark-100">Execute Procedure</h2>
          <p className="text-sm text-dark-400 mt-1">{procedure.name}</p>
        </div>

        <div className="px-6 py-6 space-y-4">
          {/* Parameters */}
          {procedure.parameters.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-dark-300 mb-3">Parameters</h3>
              <div className="space-y-3">
                {procedure.parameters.map((param) => (
                  <div key={param.name}>
                    <label className="block text-sm font-medium text-dark-400 mb-1">
                      {param.name} ({param.type})
                      {param.defaultValue && ` = ${param.defaultValue}`}
                    </label>
                    <input
                      type="text"
                      value={String(parameters[param.name] || '')}
                      onChange={(e) =>
                        setParameters({
                          ...parameters,
                          [param.name]: e.target.value,
                        })
                      }
                      className="input"
                      placeholder={param.defaultValue || `Enter ${param.name}`}
                    />
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Result */}
          {result !== null && (
            <div>
              <h3 className="text-sm font-medium text-dark-300 mb-2">Result</h3>
              <pre className="bg-dark-900 border border-dark-700 rounded-lg p-4 text-sm text-dark-200 overflow-x-auto">
                {JSON.stringify(result, null, 2)}
              </pre>
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="bg-danger-500/10 border border-danger-500/20 rounded-lg p-4">
              <p className="text-sm text-danger-400">{error}</p>
            </div>
          )}
        </div>

        <div className="border-t border-dark-700 px-6 py-4 flex items-center justify-end gap-2">
          <button onClick={onClose} className="btn-secondary" disabled={executeMutation.isPending}>
            Close
          </button>
          <button
            onClick={handleExecute}
            className="btn-primary flex items-center gap-2"
            disabled={executeMutation.isPending}
          >
            <PlayIcon className="w-4 h-4" />
            {executeMutation.isPending ? 'Executing...' : 'Execute'}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}

// Create Procedure Modal Component
function CreateProcedureModal({
  onClose,
  onCreate,
  isCreating,
}: {
  onClose: () => void;
  onCreate: (definition: CreateProcedureRequest) => void;
  isCreating: boolean;
}) {
  const [name, setName] = useState('');
  const [language, setLanguage] = useState('plpgsql');
  const [returnType, setReturnType] = useState('void');
  const [definition, setDefinition] = useState('');
  const [replace, setReplace] = useState(false);
  const [strict, setStrict] = useState(false);
  const [securityDefiner, setSecurityDefiner] = useState(false);
  const [volatility, setVolatility] = useState<'volatile' | 'stable' | 'immutable'>('volatile');

  const handleSubmit = () => {
    if (!name.trim()) {
      alert('Procedure name is required');
      return;
    }

    if (!definition.trim()) {
      alert('Procedure definition is required');
      return;
    }

    const procedureDef: CreateProcedureRequest = {
      name,
      language,
      returnType,
      definition,
      replace,
      strict,
      securityDefiner,
      volatility,
    };

    onCreate(procedureDef);
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
          <h2 className="text-xl font-semibold text-dark-100">Create Stored Procedure</h2>
        </div>

        <div className="px-6 py-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Procedure Name *
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="input"
                placeholder="my_procedure"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Language
              </label>
              <select value={language} onChange={(e) => setLanguage(e.target.value)} className="input">
                <option value="sql">SQL</option>
                <option value="plpgsql">PL/pgSQL</option>
                <option value="plpython">PL/Python</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Return Type *
              </label>
              <input
                type="text"
                value={returnType}
                onChange={(e) => setReturnType(e.target.value)}
                className="input"
                placeholder="void"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-dark-300 mb-2">
                Volatility
              </label>
              <select
                value={volatility}
                onChange={(e) => setVolatility(e.target.value as typeof volatility)}
                className="input"
              >
                <option value="volatile">Volatile</option>
                <option value="stable">Stable</option>
                <option value="immutable">Immutable</option>
              </select>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-dark-300 mb-2">
              Definition *
            </label>
            <textarea
              value={definition}
              onChange={(e) => setDefinition(e.target.value)}
              className="input font-mono text-sm"
              rows={12}
              placeholder="BEGIN&#10;  -- Your code here&#10;END;"
            />
          </div>

          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
              <input
                type="checkbox"
                checked={replace}
                onChange={(e) => setReplace(e.target.checked)}
                className="rounded border-dark-600 bg-dark-800 text-rusty-500"
              />
              Replace if exists
            </label>

            <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
              <input
                type="checkbox"
                checked={strict}
                onChange={(e) => setStrict(e.target.checked)}
                className="rounded border-dark-600 bg-dark-800 text-rusty-500"
              />
              Strict
            </label>

            <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
              <input
                type="checkbox"
                checked={securityDefiner}
                onChange={(e) => setSecurityDefiner(e.target.checked)}
                className="rounded border-dark-600 bg-dark-800 text-rusty-500"
              />
              Security Definer
            </label>
          </div>
        </div>

        <div className="border-t border-dark-700 px-6 py-4 flex items-center justify-end gap-2">
          <button onClick={onClose} className="btn-secondary" disabled={isCreating}>
            Cancel
          </button>
          <button onClick={handleSubmit} className="btn-primary" disabled={isCreating}>
            {isCreating ? 'Creating...' : 'Create Procedure'}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
