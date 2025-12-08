// ============================================================================
// Data Browser Component
// Browse and edit table data with pagination
// ============================================================================

import { useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  PencilIcon,
  TrashIcon,
  PlusIcon,
  CheckIcon,
  XMarkIcon,
  MagnifyingGlassIcon,
} from '@heroicons/react/24/outline';
import type { Column } from '../../types';
import clsx from 'clsx';

interface DataBrowserProps {
  columns: Column[];
  rows: Record<string, unknown>[];
  total: number;
  page: number;
  pageSize: number;
  onPageChange: (page: number) => void;
  onRowUpdate?: (primaryKey: Record<string, unknown>, values: Record<string, unknown>) => void;
  onRowInsert?: (values: Record<string, unknown>) => void;
  onRowDelete?: (primaryKey: Record<string, unknown>) => void;
  isLoading?: boolean;
  readOnly?: boolean;
}

export function DataBrowser({
  columns,
  rows,
  total,
  page,
  pageSize,
  onPageChange,
  onRowUpdate,
  onRowInsert,
  onRowDelete,
  isLoading = false,
  readOnly = false,
}: DataBrowserProps) {
  const [editingRow, setEditingRow] = useState<number | null>(null);
  const [editingValues, setEditingValues] = useState<Record<string, unknown>>({});
  const [isInserting, setIsInserting] = useState(false);
  const [insertValues, setInsertValues] = useState<Record<string, unknown>>({});
  const [filter, setFilter] = useState('');

  const totalPages = Math.ceil(total / pageSize);
  const primaryKeyColumns = columns.filter((col) => col.isPrimaryKey);

  const getPrimaryKey = (row: Record<string, unknown>): Record<string, unknown> => {
    const pk: Record<string, unknown> = {};
    primaryKeyColumns.forEach((col) => {
      pk[col.name] = row[col.name];
    });
    return pk;
  };

  const handleStartEdit = (index: number) => {
    setEditingRow(index);
    setEditingValues({ ...rows[index] });
  };

  const handleCancelEdit = () => {
    setEditingRow(null);
    setEditingValues({});
  };

  const handleSaveEdit = () => {
    if (editingRow !== null && onRowUpdate) {
      const pk = getPrimaryKey(rows[editingRow]);
      onRowUpdate(pk, editingValues);
      setEditingRow(null);
      setEditingValues({});
    }
  };

  const handleStartInsert = () => {
    const initialValues: Record<string, unknown> = {};
    columns.forEach((col) => {
      initialValues[col.name] = col.defaultValue || (col.nullable ? null : '');
    });
    setInsertValues(initialValues);
    setIsInserting(true);
  };

  const handleCancelInsert = () => {
    setIsInserting(false);
    setInsertValues({});
  };

  const handleSaveInsert = () => {
    if (onRowInsert) {
      onRowInsert(insertValues);
      setIsInserting(false);
      setInsertValues({});
    }
  };

  const handleDelete = (index: number) => {
    if (onRowDelete && confirm('Are you sure you want to delete this row?')) {
      const pk = getPrimaryKey(rows[index]);
      onRowDelete(pk);
    }
  };

  const formatValue = (value: unknown, column: Column): string => {
    if (value === null || value === undefined) return 'NULL';
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'object') return JSON.stringify(value);
    if (column.dataType === 'timestamp' || column.dataType === 'timestamptz') {
      return new Date(value as string).toLocaleString();
    }
    if (column.dataType === 'date') {
      return new Date(value as string).toLocaleDateString();
    }
    return String(value);
  };

  const renderCell = (
    value: unknown,
    column: Column,
    isEditing: boolean,
    onChange?: (val: unknown) => void
  ) => {
    if (!isEditing) {
      const formatted = formatValue(value, column);
      return (
        <span
          className={clsx(
            'truncate',
            value === null || value === undefined ? 'text-dark-500 italic' : 'text-dark-200'
          )}
        >
          {formatted}
        </span>
      );
    }

    // Editing mode
    if (column.isPrimaryKey) {
      return (
        <span className="text-dark-400 italic text-sm">
          {formatValue(value, column)}
        </span>
      );
    }

    if (column.dataType === 'boolean') {
      return (
        <select
          value={value === true ? 'true' : value === false ? 'false' : 'null'}
          onChange={(e) => {
            const val =
              e.target.value === 'null'
                ? null
                : e.target.value === 'true';
            onChange?.(val);
          }}
          className="input-sm w-full"
        >
          {column.nullable && <option value="null">NULL</option>}
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      );
    }

    if (column.dataType === 'text' || column.dataType === 'json' || column.dataType === 'jsonb') {
      return (
        <textarea
          value={value === null ? '' : String(value)}
          onChange={(e) => onChange?.(e.target.value || null)}
          className="input-sm w-full"
          rows={2}
        />
      );
    }

    return (
      <input
        type={column.dataType === 'integer' || column.dataType === 'bigint' ? 'number' : 'text'}
        value={value === null ? '' : String(value)}
        onChange={(e) => {
          const val = e.target.value;
          if (column.dataType === 'integer' || column.dataType === 'bigint') {
            onChange?.(val ? parseInt(val) : null);
          } else if (column.dataType === 'decimal' || column.dataType === 'numeric') {
            onChange?.(val ? parseFloat(val) : null);
          } else {
            onChange?.(val || null);
          }
        }}
        className="input-sm w-full"
      />
    );
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-dark-400">
          <div className="w-5 h-5 border-2 border-dark-400 border-t-rusty-500 rounded-full animate-spin" />
          <span>Loading data...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Toolbar */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2 flex-1">
          <MagnifyingGlassIcon className="w-5 h-5 text-dark-400" />
          <input
            type="text"
            placeholder="Filter rows..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="input flex-1 max-w-md"
          />
        </div>

        {!readOnly && onRowInsert && (
          <button onClick={handleStartInsert} className="btn-primary flex items-center gap-2">
            <PlusIcon className="w-4 h-4" />
            Insert Row
          </button>
        )}
      </div>

      {/* Data Table */}
      <div className="overflow-x-auto border border-dark-700 rounded-lg">
        <table className="w-full">
          <thead>
            <tr className="bg-dark-800 border-b border-dark-700">
              {columns.map((column) => (
                <th
                  key={column.name}
                  className="text-left py-3 px-4 text-sm font-medium text-dark-400 whitespace-nowrap"
                >
                  <div className="flex items-center gap-2">
                    <span>{column.name}</span>
                    {column.isPrimaryKey && (
                      <span className="text-xs text-blue-400">PK</span>
                    )}
                    {column.isForeignKey && (
                      <span className="text-xs text-purple-400">FK</span>
                    )}
                  </div>
                  <div className="text-xs text-dark-500 font-normal mt-0.5">
                    {column.dataType}
                  </div>
                </th>
              ))}
              {!readOnly && (onRowUpdate || onRowDelete) && (
                <th className="text-right py-3 px-4 text-sm font-medium text-dark-400 whitespace-nowrap">
                  Actions
                </th>
              )}
            </tr>
          </thead>
          <tbody>
            {/* Insert Row */}
            {isInserting && (
              <tr className="bg-rusty-500/5 border-b border-rusty-500/20">
                {columns.map((column) => (
                  <td key={column.name} className="py-2 px-4">
                    {renderCell(insertValues[column.name], column, true, (val) =>
                      setInsertValues({ ...insertValues, [column.name]: val })
                    )}
                  </td>
                ))}
                <td className="py-2 px-4">
                  <div className="flex items-center justify-end gap-1">
                    <button
                      onClick={handleSaveInsert}
                      className="p-1.5 rounded hover:bg-dark-700 text-success-400 hover:text-success-300"
                      title="Save"
                    >
                      <CheckIcon className="w-4 h-4" />
                    </button>
                    <button
                      onClick={handleCancelInsert}
                      className="p-1.5 rounded hover:bg-dark-700 text-danger-400 hover:text-danger-300"
                      title="Cancel"
                    >
                      <XMarkIcon className="w-4 h-4" />
                    </button>
                  </div>
                </td>
              </tr>
            )}

            {/* Data Rows */}
            {rows.map((row, index) => {
              const isEditing = editingRow === index;
              return (
                <motion.tr
                  key={index}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className={clsx(
                    'border-b border-dark-700/50 hover:bg-dark-800/30',
                    isEditing && 'bg-rusty-500/5'
                  )}
                >
                  {columns.map((column) => (
                    <td key={column.name} className="py-2 px-4 max-w-xs">
                      {renderCell(
                        isEditing ? editingValues[column.name] : row[column.name],
                        column,
                        isEditing,
                        (val) => setEditingValues({ ...editingValues, [column.name]: val })
                      )}
                    </td>
                  ))}
                  {!readOnly && (onRowUpdate || onRowDelete) && (
                    <td className="py-2 px-4">
                      <div className="flex items-center justify-end gap-1">
                        {isEditing ? (
                          <>
                            <button
                              onClick={handleSaveEdit}
                              className="p-1.5 rounded hover:bg-dark-700 text-success-400 hover:text-success-300"
                              title="Save"
                            >
                              <CheckIcon className="w-4 h-4" />
                            </button>
                            <button
                              onClick={handleCancelEdit}
                              className="p-1.5 rounded hover:bg-dark-700 text-danger-400 hover:text-danger-300"
                              title="Cancel"
                            >
                              <XMarkIcon className="w-4 h-4" />
                            </button>
                          </>
                        ) : (
                          <>
                            {onRowUpdate && (
                              <button
                                onClick={() => handleStartEdit(index)}
                                className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200"
                                title="Edit"
                              >
                                <PencilIcon className="w-4 h-4" />
                              </button>
                            )}
                            {onRowDelete && (
                              <button
                                onClick={() => handleDelete(index)}
                                className="p-1.5 rounded hover:bg-dark-700 text-dark-400 hover:text-danger-400"
                                title="Delete"
                              >
                                <TrashIcon className="w-4 h-4" />
                              </button>
                            )}
                          </>
                        )}
                      </div>
                    </td>
                  )}
                </motion.tr>
              );
            })}
          </tbody>
        </table>

        {rows.length === 0 && !isInserting && (
          <div className="text-center py-12 text-dark-400">
            <p>No data found</p>
          </div>
        )}
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between">
        <div className="text-sm text-dark-400">
          Showing {(page - 1) * pageSize + 1} to {Math.min(page * pageSize, total)} of {total}{' '}
          rows
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => onPageChange(page - 1)}
            disabled={page === 1}
            className="btn-secondary flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <ChevronLeftIcon className="w-4 h-4" />
            Previous
          </button>

          <div className="flex items-center gap-1">
            {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
              let pageNum: number;
              if (totalPages <= 5) {
                pageNum = i + 1;
              } else if (page <= 3) {
                pageNum = i + 1;
              } else if (page >= totalPages - 2) {
                pageNum = totalPages - 4 + i;
              } else {
                pageNum = page - 2 + i;
              }

              return (
                <button
                  key={pageNum}
                  onClick={() => onPageChange(pageNum)}
                  className={clsx(
                    'px-3 py-1.5 rounded text-sm font-medium transition-colors',
                    page === pageNum
                      ? 'bg-rusty-500 text-white'
                      : 'bg-dark-800 text-dark-300 hover:bg-dark-700'
                  )}
                >
                  {pageNum}
                </button>
              );
            })}
          </div>

          <button
            onClick={() => onPageChange(page + 1)}
            disabled={page === totalPages}
            className="btn-secondary flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Next
            <ChevronRightIcon className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
