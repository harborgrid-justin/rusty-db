// ============================================================================
// Table Management Page
// Main page for managing database tables
// ============================================================================

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  PlusIcon,
  ArrowPathIcon,
  FunnelIcon,
  TableCellsIcon,
  CircleStackIcon,
  DocumentArrowDownIcon,
  DocumentArrowUpIcon,
  MapIcon,
} from '@heroicons/react/24/outline';
import { useTables, useDropTable, useCreateTable } from '../hooks/useSchema';
import { TableList } from '../components/schema/TableList';
import { CreateTableWizard } from '../components/schema/CreateTableWizard';
import { RelationshipDiagram } from '../components/schema/RelationshipDiagram';
import { useNavigate } from 'react-router-dom';
import type { Table } from '../types';
import type { CreateTableRequest } from '../services/schemaService';
import { getErrorMessage } from '../services/api';
import clsx from 'clsx';

type ViewMode = 'list' | 'diagram';

export default function TableManagement() {
  const navigate = useNavigate();
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [showCreateWizard, setShowCreateWizard] = useState(false);
  const [showFilters, setShowFilters] = useState(false);
  const [schemaFilter, setSchemaFilter] = useState('public');
  const [searchTerm, setSearchTerm] = useState('');
  const [includeSystem, setIncludeSystem] = useState(false);

  const {
    data: tablesData,
    isLoading,
    refetch,
  } = useTables({
    schema: schemaFilter,
    search: searchTerm || undefined,
    includeSystem,
    page: 1,
    pageSize: 100,
  });

  const dropTableMutation = useDropTable();
  const createTableMutation = useCreateTable();

  const tables = tablesData?.data || [];
  const totalTables = tablesData?.total || 0;

  const handleCreateTable = async (definition: CreateTableRequest) => {
    try {
      await createTableMutation.mutateAsync(definition);
      setShowCreateWizard(false);
      // Show success notification
      alert('Table created successfully!');
    } catch (error) {
      alert(`Failed to create table: ${getErrorMessage(error)}`);
    }
  };

  const handleDropTable = async (table: Table) => {
    if (
      !confirm(
        `Are you sure you want to drop table "${table.name}"?\n\nThis action cannot be undone.`
      )
    ) {
      return;
    }

    const cascade = confirm(
      'Drop dependent objects as well (CASCADE)?'
    );

    try {
      await dropTableMutation.mutateAsync({
        tableName: table.name,
        schema: table.schema,
        cascade,
      });
      alert('Table dropped successfully');
    } catch (error) {
      alert(`Failed to drop table: ${getErrorMessage(error)}`);
    }
  };

  const handleAnalyzeTable = async (table: Table) => {
    try {
      // In a real implementation, this would call an analyze endpoint
      alert(`Analyzing table ${table.name}...`);
    } catch (error) {
      alert(`Failed to analyze table: ${getErrorMessage(error)}`);
    }
  };

  const handleExportSchema = () => {
    // Placeholder for export functionality
    alert('Export schema functionality would be implemented here');
  };

  const handleImportSchema = () => {
    // Placeholder for import functionality
    alert('Import schema functionality would be implemented here');
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <div className="flex items-center gap-3 mb-2">
          <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
            <TableCellsIcon className="w-6 h-6 text-rusty-400" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-dark-100">Tables</h1>
            <p className="text-sm text-dark-400">
              Manage database tables, columns, and schemas
            </p>
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Tables</p>
              <p className="text-2xl font-bold text-dark-100">{totalTables}</p>
            </div>
            <div className="w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center">
              <CircleStackIcon className="w-6 h-6 text-blue-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Rows</p>
              <p className="text-2xl font-bold text-dark-100">
                {new Intl.NumberFormat().format(
                  tables.reduce((sum, t) => sum + t.rowCount, 0)
                )}
              </p>
            </div>
            <div className="w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center">
              <TableCellsIcon className="w-6 h-6 text-green-400" />
            </div>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-dark-400 mb-1">Total Size</p>
              <p className="text-2xl font-bold text-dark-100">
                {(() => {
                  const totalBytes = tables.reduce((sum, t) => sum + t.size, 0);
                  const gb = totalBytes / (1024 * 1024 * 1024);
                  return gb >= 1
                    ? `${gb.toFixed(2)} GB`
                    : `${(totalBytes / (1024 * 1024)).toFixed(2)} MB`;
                })()}
              </p>
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
          {/* View Mode Toggle */}
          <div className="flex items-center gap-1 p-1 bg-dark-800 border border-dark-700 rounded-lg">
            <button
              onClick={() => setViewMode('list')}
              className={clsx(
                'px-3 py-1.5 rounded text-sm font-medium transition-colors',
                viewMode === 'list'
                  ? 'bg-rusty-500 text-white'
                  : 'text-dark-400 hover:text-dark-200'
              )}
            >
              <TableCellsIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => setViewMode('diagram')}
              className={clsx(
                'px-3 py-1.5 rounded text-sm font-medium transition-colors',
                viewMode === 'diagram'
                  ? 'bg-rusty-500 text-white'
                  : 'text-dark-400 hover:text-dark-200'
              )}
            >
              <MapIcon className="w-4 h-4" />
            </button>
          </div>

          {/* Filters */}
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
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleExportSchema}
            className="btn-secondary flex items-center gap-2"
          >
            <DocumentArrowDownIcon className="w-4 h-4" />
            Export
          </button>

          <button
            onClick={handleImportSchema}
            className="btn-secondary flex items-center gap-2"
          >
            <DocumentArrowUpIcon className="w-4 h-4" />
            Import
          </button>

          <button
            onClick={() => setShowCreateWizard(true)}
            className="btn-primary flex items-center gap-2"
          >
            <PlusIcon className="w-4 h-4" />
            Create Table
          </button>
        </div>
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
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Schema
                </label>
                <input
                  type="text"
                  value={schemaFilter}
                  onChange={(e) => setSchemaFilter(e.target.value)}
                  className="input"
                  placeholder="public"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Search
                </label>
                <input
                  type="text"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="input"
                  placeholder="Search tables..."
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Options
                </label>
                <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={includeSystem}
                    onChange={(e) => setIncludeSystem(e.target.checked)}
                    className="rounded border-dark-600 bg-dark-800 text-rusty-500"
                  />
                  Include system tables
                </label>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Content */}
      <div className="card">
        {viewMode === 'list' ? (
          <TableList
            tables={tables}
            onTableDelete={handleDropTable}
            onTableRefresh={handleAnalyzeTable}
            onTableSelect={(table) => navigate(`/tables/${table.name}`)}
            isLoading={isLoading}
          />
        ) : (
          <RelationshipDiagram
            tables={tables}
            onTableClick={(tableName) => navigate(`/tables/${tableName}`)}
          />
        )}
      </div>

      {/* Create Table Wizard Modal */}
      <AnimatePresence>
        {showCreateWizard && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
            onClick={() => setShowCreateWizard(false)}
          >
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-4xl max-h-[90vh] overflow-y-auto"
            >
              <CreateTableWizard
                onSubmit={handleCreateTable}
                onCancel={() => setShowCreateWizard(false)}
                isSubmitting={createTableMutation.isPending}
              />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
