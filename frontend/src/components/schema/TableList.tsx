// ============================================================================
// Table List Component
// Displays a list of database tables with actions
// ============================================================================

import { useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import {
  TableCellsIcon,
  MagnifyingGlassIcon,
  EyeIcon,
  PencilIcon,
  TrashIcon,
  ArrowPathIcon,
  ChartBarIcon,
} from '@heroicons/react/24/outline';
import type { Table } from '../../types';
import clsx from 'clsx';

interface TableListProps {
  tables: Table[];
  onTableSelect?: (table: Table) => void;
  onTableDelete?: (table: Table) => void;
  onTableRefresh?: (table: Table) => void;
  isLoading?: boolean;
}

export function TableList({
  tables,
  onTableSelect,
  onTableDelete,
  onTableRefresh,
  isLoading = false,
}: TableListProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<'name' | 'size' | 'rows'>('name');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('asc');

  const filteredTables = tables.filter((table) =>
    table.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const sortedTables = [...filteredTables].sort((a, b) => {
    let comparison = 0;
    switch (sortBy) {
      case 'name':
        comparison = a.name.localeCompare(b.name);
        break;
      case 'size':
        comparison = a.size - b.size;
        break;
      case 'rows':
        comparison = a.rowCount - b.rowCount;
        break;
    }
    return sortOrder === 'asc' ? comparison : -comparison;
  });

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  const formatNumber = (num: number): string => {
    return new Intl.NumberFormat().format(num);
  };

  const formatDate = (date: string): string => {
    return new Date(date).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const handleSort = (column: 'name' | 'size' | 'rows') => {
    if (sortBy === column) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(column);
      setSortOrder('asc');
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-dark-400">
          <ArrowPathIcon className="w-5 h-5 animate-spin" />
          <span>Loading tables...</span>
        </div>
      </div>
    );
  }

  if (tables.length === 0) {
    return (
      <div className="card text-center py-12">
        <TableCellsIcon className="w-16 h-16 text-dark-400 mx-auto mb-4" />
        <h3 className="text-lg font-medium text-dark-200 mb-2">No Tables Found</h3>
        <p className="text-dark-400 mb-6">
          Get started by creating your first table
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Search */}
      <div className="flex items-center gap-2 px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg">
        <MagnifyingGlassIcon className="w-5 h-5 text-dark-400" />
        <input
          type="text"
          placeholder="Search tables..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="flex-1 bg-transparent border-none outline-none text-dark-200 placeholder-dark-400"
        />
      </div>

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="border-b border-dark-700">
              <th
                className="text-left py-3 px-4 text-sm font-medium text-dark-400 cursor-pointer hover:text-dark-200"
                onClick={() => handleSort('name')}
              >
                <div className="flex items-center gap-2">
                  <span>Name</span>
                  {sortBy === 'name' && (
                    <span className="text-rusty-400">
                      {sortOrder === 'asc' ? '↑' : '↓'}
                    </span>
                  )}
                </div>
              </th>
              <th className="text-left py-3 px-4 text-sm font-medium text-dark-400">
                Schema
              </th>
              <th
                className="text-right py-3 px-4 text-sm font-medium text-dark-400 cursor-pointer hover:text-dark-200"
                onClick={() => handleSort('rows')}
              >
                <div className="flex items-center justify-end gap-2">
                  <span>Rows</span>
                  {sortBy === 'rows' && (
                    <span className="text-rusty-400">
                      {sortOrder === 'asc' ? '↑' : '↓'}
                    </span>
                  )}
                </div>
              </th>
              <th
                className="text-right py-3 px-4 text-sm font-medium text-dark-400 cursor-pointer hover:text-dark-200"
                onClick={() => handleSort('size')}
              >
                <div className="flex items-center justify-end gap-2">
                  <span>Size</span>
                  {sortBy === 'size' && (
                    <span className="text-rusty-400">
                      {sortOrder === 'asc' ? '↑' : '↓'}
                    </span>
                  )}
                </div>
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
            {sortedTables.map((table, index) => (
              <motion.tr
                key={table.name}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.02 }}
                className="border-b border-dark-700/50 hover:bg-dark-800/50 transition-colors"
              >
                <td className="py-3 px-4">
                  <Link
                    to={`/tables/${table.name}`}
                    className="flex items-center gap-2 text-dark-200 hover:text-rusty-400 transition-colors"
                  >
                    <TableCellsIcon className="w-4 h-4" />
                    <span className="font-medium">{table.name}</span>
                  </Link>
                </td>
                <td className="py-3 px-4 text-dark-400 text-sm">{table.schema}</td>
                <td className="py-3 px-4 text-right text-dark-200 text-sm">
                  {formatNumber(table.rowCount)}
                </td>
                <td className="py-3 px-4 text-right text-dark-200 text-sm">
                  {formatBytes(table.size)}
                </td>
                <td className="py-3 px-4 text-dark-400 text-sm">
                  {formatDate(table.updatedAt)}
                </td>
                <td className="py-3 px-4">
                  <div className="flex items-center justify-end gap-1">
                    <Link
                      to={`/tables/${table.name}`}
                      className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
                      title="View Table"
                    >
                      <EyeIcon className="w-4 h-4" />
                    </Link>
                    {onTableRefresh && (
                      <button
                        onClick={() => onTableRefresh(table)}
                        className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
                        title="Analyze Table"
                      >
                        <ChartBarIcon className="w-4 h-4" />
                      </button>
                    )}
                    {onTableDelete && (
                      <button
                        onClick={() => onTableDelete(table)}
                        className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-danger-400 transition-colors"
                        title="Drop Table"
                      >
                        <TrashIcon className="w-4 h-4" />
                      </button>
                    )}
                  </div>
                </td>
              </motion.tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Summary */}
      <div className="flex items-center justify-between text-sm text-dark-400 pt-2">
        <span>
          Showing {sortedTables.length} of {tables.length} tables
        </span>
        <span>
          Total size: {formatBytes(tables.reduce((sum, t) => sum + t.size, 0))}
        </span>
      </div>
    </div>
  );
}
