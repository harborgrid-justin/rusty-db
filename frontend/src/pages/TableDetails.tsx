// ============================================================================
// Table Details Page
// Individual table details with columns, indexes, data, etc.
// ============================================================================

import { useState } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ArrowLeftIcon,
  TableCellsIcon,
  QueueListIcon,
  KeyIcon,
  CircleStackIcon,
  CodeBracketIcon,
  ChartBarIcon,
  TrashIcon,
  PencilIcon,
  ArrowPathIcon,
} from '@heroicons/react/24/outline';
import {
  useTable,
  useTableStats,
  useTableDDL,
  useForeignKeys,
  useConstraints,
  useBrowseTableData,
  useUpdateRow,
  useInsertRow,
  useDeleteRow,
  useDropTable,
} from '../hooks/useSchema';
import { ColumnEditor } from '../components/schema/ColumnEditor';
import { IndexList } from '../components/schema/IndexList';
import { DataBrowser } from '../components/schema/DataBrowser';
import { DDLViewer } from '../components/schema/DDLViewer';
import clsx from 'clsx';
import type { ColumnDefinition, UpdateRowRequest, InsertRowRequest } from '../services/schemaService';
import { getErrorMessage } from '../services/api';

type TabType = 'columns' | 'indexes' | 'foreignKeys' | 'data' | 'ddl' | 'stats';

export default function TableDetails() {
  const { tableName } = useParams<{ tableName: string }>();
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState<TabType>('columns');
  const [dataPage, setDataPage] = useState(1);
  const [dataPageSize] = useState(50);

  const { data: table, isLoading: tableLoading } = useTable(tableName!, 'public');
  const { data: stats } = useTableStats(tableName!, 'public');
  const { data: ddlData } = useTableDDL(tableName!, 'public');
  const { data: foreignKeys = [] } = useForeignKeys(tableName!, 'public');
  const { data: constraints = [] } = useConstraints(tableName!, 'public');
  const { data: browseData } = useBrowseTableData(
    tableName!,
    { page: dataPage, pageSize: dataPageSize },
    'public'
  );

  const updateRowMutation = useUpdateRow(tableName!, 'public');
  const insertRowMutation = useInsertRow(tableName!, 'public');
  const deleteRowMutation = useDeleteRow(tableName!, 'public');
  const dropTableMutation = useDropTable();

  const handleDropTable = async () => {
    if (
      !confirm(
        `Are you sure you want to drop table "${tableName}"?\n\nThis action cannot be undone.`
      )
    ) {
      return;
    }

    const cascade = confirm('Drop dependent objects as well (CASCADE)?');

    try {
      await dropTableMutation.mutateAsync({
        tableName: tableName!,
        schema: 'public',
        cascade,
      });
      navigate('/tables');
    } catch (error) {
      alert(`Failed to drop table: ${getErrorMessage(error)}`);
    }
  };

  const handleUpdateRow = async (primaryKey: Record<string, unknown>, values: Record<string, unknown>) => {
    try {
      await updateRowMutation.mutateAsync({ primaryKey, values });
      alert('Row updated successfully');
    } catch (error) {
      alert(`Failed to update row: ${getErrorMessage(error)}`);
    }
  };

  const handleInsertRow = async (values: Record<string, unknown>) => {
    try {
      await insertRowMutation.mutateAsync({ values });
      alert('Row inserted successfully');
    } catch (error) {
      alert(`Failed to insert row: ${getErrorMessage(error)}`);
    }
  };

  const handleDeleteRow = async (primaryKey: Record<string, unknown>) => {
    try {
      await deleteRowMutation.mutateAsync(primaryKey);
      alert('Row deleted successfully');
    } catch (error) {
      alert(`Failed to delete row: ${getErrorMessage(error)}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (!bytes) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  const formatNumber = (num: number): string => {
    return new Intl.NumberFormat().format(num);
  };

  if (tableLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-dark-400">
          <div className="w-6 h-6 border-2 border-dark-400 border-t-rusty-500 rounded-full animate-spin" />
          <span>Loading table details...</span>
        </div>
      </div>
    );
  }

  if (!table) {
    return (
      <div className="card text-center py-12">
        <TableCellsIcon className="w-16 h-16 text-dark-400 mx-auto mb-4" />
        <h3 className="text-lg font-medium text-dark-200 mb-2">Table Not Found</h3>
        <p className="text-dark-400 mb-6">
          The table "{tableName}" could not be found
        </p>
        <Link to="/tables" className="btn-primary">
          Back to Tables
        </Link>
      </div>
    );
  }

  const tabs = [
    { id: 'columns' as TabType, label: 'Columns', icon: TableCellsIcon, count: table.columns.length },
    { id: 'indexes' as TabType, label: 'Indexes', icon: QueueListIcon, count: table.indexes.length },
    { id: 'foreignKeys' as TabType, label: 'Foreign Keys', icon: KeyIcon, count: foreignKeys.length },
    { id: 'data' as TabType, label: 'Data', icon: CircleStackIcon },
    { id: 'ddl' as TabType, label: 'DDL', icon: CodeBracketIcon },
    { id: 'stats' as TabType, label: 'Statistics', icon: ChartBarIcon },
  ];

  return (
    <div className="space-y-6">
      {/* Breadcrumb */}
      <div className="flex items-center gap-2 text-sm">
        <Link to="/tables" className="text-dark-400 hover:text-dark-200 transition-colors">
          Tables
        </Link>
        <span className="text-dark-600">/</span>
        <span className="text-dark-200">{tableName}</span>
      </div>

      {/* Page Header */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-4">
          <Link
            to="/tables"
            className="w-10 h-10 bg-dark-800 border border-dark-700 rounded-lg flex items-center justify-center hover:bg-dark-700 transition-colors"
          >
            <ArrowLeftIcon className="w-5 h-5 text-dark-300" />
          </Link>
          <div>
            <div className="flex items-center gap-3 mb-2">
              <TableCellsIcon className="w-8 h-8 text-rusty-400" />
              <h1 className="text-2xl font-bold text-dark-100">{table.name}</h1>
            </div>
            <p className="text-sm text-dark-400">
              Schema: {table.schema} • {table.columns.length} columns • {formatNumber(table.rowCount)} rows • {formatBytes(table.size)}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button className="btn-secondary flex items-center gap-2">
            <ArrowPathIcon className="w-4 h-4" />
            Analyze
          </button>
          <button className="btn-secondary flex items-center gap-2">
            <PencilIcon className="w-4 h-4" />
            Alter
          </button>
          <button
            onClick={handleDropTable}
            className="btn-danger flex items-center gap-2"
          >
            <TrashIcon className="w-4 h-4" />
            Drop
          </button>
        </div>
      </div>

      {/* Quick Stats */}
      {stats && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="card">
            <p className="text-sm text-dark-400 mb-1">Row Count</p>
            <p className="text-xl font-bold text-dark-100">{formatNumber(stats.rowCount)}</p>
          </div>
          <div className="card">
            <p className="text-sm text-dark-400 mb-1">Table Size</p>
            <p className="text-xl font-bold text-dark-100">{formatBytes(stats.size)}</p>
          </div>
          <div className="card">
            <p className="text-sm text-dark-400 mb-1">Index Size</p>
            <p className="text-xl font-bold text-dark-100">{formatBytes(stats.indexSize)}</p>
          </div>
          <div className="card">
            <p className="text-sm text-dark-400 mb-1">Total Size</p>
            <p className="text-xl font-bold text-dark-100">{formatBytes(stats.totalSize)}</p>
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="border-b border-dark-700">
        <div className="flex items-center gap-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={clsx(
                'flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors',
                activeTab === tab.id
                  ? 'border-rusty-500 text-rusty-400'
                  : 'border-transparent text-dark-400 hover:text-dark-200 hover:border-dark-600'
              )}
            >
              <tab.icon className="w-4 h-4" />
              <span>{tab.label}</span>
              {tab.count !== undefined && (
                <span className={clsx(
                  'px-2 py-0.5 rounded-full text-xs',
                  activeTab === tab.id
                    ? 'bg-rusty-500/20 text-rusty-400'
                    : 'bg-dark-700 text-dark-400'
                )}>
                  {tab.count}
                </span>
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Tab Content */}
      <AnimatePresence mode="wait">
        <motion.div
          key={activeTab}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          transition={{ duration: 0.2 }}
        >
          {activeTab === 'columns' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold text-dark-100">Columns</h2>
              </div>
              <div className="card">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Name</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Type</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Nullable</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Default</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Constraints</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Comment</th>
                    </tr>
                  </thead>
                  <tbody>
                    {table.columns.map((column, index) => (
                      <tr key={column.name} className="border-b border-dark-700/50 hover:bg-dark-800/30">
                        <td className="py-3 px-4">
                          <div className="flex items-center gap-2">
                            <span className="text-dark-200 font-medium">{column.name}</span>
                            {column.isPrimaryKey && (
                              <span className="px-2 py-0.5 bg-blue-500/20 border border-blue-500/30 rounded text-xs text-blue-400">
                                PK
                              </span>
                            )}
                            {column.isForeignKey && (
                              <span className="px-2 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded text-xs text-purple-400">
                                FK
                              </span>
                            )}
                          </div>
                        </td>
                        <td className="py-3 px-4 text-dark-300 font-mono text-sm">
                          {column.dataType.toUpperCase()}
                        </td>
                        <td className="py-3 px-4">
                          {column.nullable ? (
                            <span className="text-dark-400">YES</span>
                          ) : (
                            <span className="text-rusty-400">NO</span>
                          )}
                        </td>
                        <td className="py-3 px-4 text-dark-400 text-sm font-mono">
                          {column.defaultValue || '-'}
                        </td>
                        <td className="py-3 px-4">
                          <div className="flex flex-wrap gap-1">
                            {column.isUnique && (
                              <span className="px-2 py-0.5 bg-dark-700 rounded text-xs text-dark-300">
                                UNIQUE
                              </span>
                            )}
                            {column.isIndexed && (
                              <span className="px-2 py-0.5 bg-dark-700 rounded text-xs text-dark-300">
                                INDEXED
                              </span>
                            )}
                          </div>
                        </td>
                        <td className="py-3 px-4 text-dark-400 text-sm">
                          {column.comment || '-'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {activeTab === 'indexes' && (
            <div className="card">
              <IndexList
                indexes={table.indexes}
                showTable={false}
              />
            </div>
          )}

          {activeTab === 'foreignKeys' && (
            <div className="card">
              {foreignKeys.length === 0 ? (
                <div className="text-center py-12">
                  <KeyIcon className="w-12 h-12 text-dark-400 mx-auto mb-3" />
                  <p className="text-dark-400">No foreign keys defined</p>
                </div>
              ) : (
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Name</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">Columns</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">References</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">On Delete</th>
                      <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">On Update</th>
                    </tr>
                  </thead>
                  <tbody>
                    {foreignKeys.map((fk) => (
                      <tr key={fk.name} className="border-b border-dark-700/50">
                        <td className="py-3 px-4 text-dark-200 font-medium">{fk.name}</td>
                        <td className="py-3 px-4 text-dark-300">{fk.columns.join(', ')}</td>
                        <td className="py-3 px-4 text-dark-300">
                          {fk.referencedTable} ({fk.referencedColumns.join(', ')})
                        </td>
                        <td className="py-3 px-4 text-dark-400 text-sm">{fk.onDelete.toUpperCase()}</td>
                        <td className="py-3 px-4 text-dark-400 text-sm">{fk.onUpdate.toUpperCase()}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          )}

          {activeTab === 'data' && browseData && (
            <DataBrowser
              columns={browseData.columns}
              rows={browseData.rows}
              total={browseData.total}
              page={dataPage}
              pageSize={dataPageSize}
              onPageChange={setDataPage}
              onRowUpdate={handleUpdateRow}
              onRowInsert={handleInsertRow}
              onRowDelete={handleDeleteRow}
            />
          )}

          {activeTab === 'ddl' && ddlData && (
            <DDLViewer ddl={ddlData.ddl} title={`DDL for ${table.name}`} />
          )}

          {activeTab === 'stats' && stats && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="card">
                <h3 className="text-sm font-medium text-dark-300 mb-4">Storage Information</h3>
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Row Count</span>
                    <span className="text-sm font-medium text-dark-200">{formatNumber(stats.rowCount)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Table Size</span>
                    <span className="text-sm font-medium text-dark-200">{formatBytes(stats.size)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Index Size</span>
                    <span className="text-sm font-medium text-dark-200">{formatBytes(stats.indexSize)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Total Size</span>
                    <span className="text-sm font-medium text-dark-200">{formatBytes(stats.totalSize)}</span>
                  </div>
                </div>
              </div>

              <div className="card">
                <h3 className="text-sm font-medium text-dark-300 mb-4">Maintenance Information</h3>
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Last Vacuum</span>
                    <span className="text-sm font-medium text-dark-200">
                      {stats.lastVacuum ? new Date(stats.lastVacuum).toLocaleString() : 'Never'}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Last Analyze</span>
                    <span className="text-sm font-medium text-dark-200">
                      {stats.lastAnalyze ? new Date(stats.lastAnalyze).toLocaleString() : 'Never'}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Last Auto Vacuum</span>
                    <span className="text-sm font-medium text-dark-200">
                      {stats.lastAutoVacuum ? new Date(stats.lastAutoVacuum).toLocaleString() : 'Never'}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-dark-400">Last Auto Analyze</span>
                    <span className="text-sm font-medium text-dark-200">
                      {stats.lastAutoAnalyze ? new Date(stats.lastAutoAnalyze).toLocaleString() : 'Never'}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
