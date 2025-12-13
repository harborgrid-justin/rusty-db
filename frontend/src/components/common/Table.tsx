import { ReactNode } from 'react';
import { ChevronUpIcon, ChevronDownIcon } from '@heroicons/react/24/outline';
import { LoadingSpinner } from './LoadingScreen';

// ============================================================================
// Table Component
// Data table with sorting, selection, and pagination
// ============================================================================

export interface Column<T = any> {
  key: string;
  header: string | ReactNode;
  render?: (value: any, row: T, index: number) => ReactNode;
  sortable?: boolean;
  width?: string;
  align?: 'left' | 'center' | 'right';
}

export interface TableProps<T = any> {
  columns: Column<T>[];
  data: T[];
  loading?: boolean;
  emptyMessage?: string;
  keyField?: string;
  onRowClick?: (row: T, index: number) => void;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
  onSort?: (key: string) => void;
  selectable?: boolean;
  selectedRows?: Set<string>;
  onSelectRow?: (key: string) => void;
  onSelectAll?: () => void;
  className?: string;
  striped?: boolean;
  hoverable?: boolean;
  compact?: boolean;
}

export function Table<T extends Record<string, any>>({
  columns,
  data,
  loading = false,
  emptyMessage = 'No data available',
  keyField = 'id',
  onRowClick,
  sortBy,
  sortOrder,
  onSort,
  selectable = false,
  selectedRows,
  onSelectRow,
  onSelectAll,
  className = '',
  striped = true,
  hoverable = true,
  compact = false,
}: TableProps<T>) {
  const handleSort = (key: string, sortable?: boolean) => {
    if (sortable && onSort) {
      onSort(key);
    }
  };

  const isAllSelected = selectable && selectedRows && data.length > 0 &&
    data.every(row => selectedRows.has(row[keyField]));

  return (
    <div className={`overflow-x-auto ${className}`}>
      <table className="w-full border-collapse">
        <thead className="bg-white/5 backdrop-blur-sm border-b border-white/10">
          <tr>
            {selectable && (
              <th className={`${compact ? 'px-3 py-2' : 'px-4 py-3'} text-left`}>
                <input
                  type="checkbox"
                  checked={isAllSelected}
                  onChange={onSelectAll}
                  className="rounded border-dark-600 bg-dark-700 text-primary-500 focus:ring-primary-500 focus:ring-offset-dark-800"
                />
              </th>
            )}
            {columns.map((column) => (
              <th
                key={column.key}
                className={`${compact ? 'px-3 py-2' : 'px-4 py-3'} text-${column.align || 'left'} text-xs font-semibold uppercase tracking-wider text-dark-400 ${
                  column.sortable ? 'cursor-pointer select-none hover:text-dark-100' : ''
                }`}
                style={{ width: column.width }}
                onClick={() => handleSort(column.key, column.sortable)}
              >
                <div className="flex items-center gap-2">
                  {column.header}
                  {column.sortable && (
                    <div className="flex flex-col">
                      {sortBy === column.key ? (
                        sortOrder === 'asc' ? (
                          <ChevronUpIcon className="w-3 h-3 text-primary-400" />
                        ) : (
                          <ChevronDownIcon className="w-3 h-3 text-primary-400" />
                        )
                      ) : (
                        <div className="text-dark-600">
                          <ChevronUpIcon className="w-3 h-3" />
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-white/5">
          {loading ? (
            <tr>
              <td colSpan={columns.length + (selectable ? 1 : 0)} className="py-12">
                <div className="flex justify-center">
                  <LoadingSpinner size="lg" />
                </div>
              </td>
            </tr>
          ) : data.length === 0 ? (
            <tr>
              <td
                colSpan={columns.length + (selectable ? 1 : 0)}
                className="py-12 text-center text-dark-400"
              >
                {emptyMessage}
              </td>
            </tr>
          ) : (
            data.map((row, index) => {
              const rowKey = row[keyField];
              const isSelected = selectedRows?.has(rowKey);

              return (
                <tr
                  key={rowKey}
                  className={`
                    ${striped && index % 2 === 0 ? 'bg-white/[0.02]' : ''}
                    ${hoverable ? 'hover:bg-white/5' : ''}
                    ${onRowClick ? 'cursor-pointer' : ''}
                    ${isSelected ? 'bg-primary-500/10' : ''}
                    transition-colors
                  `}
                  onClick={() => onRowClick?.(row, index)}
                >
                  {selectable && (
                    <td className={`${compact ? 'px-3 py-2' : 'px-4 py-3'}`}>
                      <input
                        type="checkbox"
                        checked={isSelected}
                        onChange={(e) => {
                          e.stopPropagation();
                          onSelectRow?.(rowKey);
                        }}
                        onClick={(e) => e.stopPropagation()}
                        className="rounded border-dark-600 bg-dark-700 text-primary-500 focus:ring-primary-500 focus:ring-offset-dark-800"
                      />
                    </td>
                  )}
                  {columns.map((column) => (
                    <td
                      key={column.key}
                      className={`${compact ? 'px-3 py-2' : 'px-4 py-3'} text-${column.align || 'left'} text-sm text-dark-200`}
                    >
                      {column.render
                        ? column.render(row[column.key], row, index)
                        : row[column.key]}
                    </td>
                  ))}
                </tr>
              );
            })
          )}
        </tbody>
      </table>
    </div>
  );
}

// Simple pagination component
export interface PaginationProps {
  currentPage: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  pageSize?: number;
  totalItems?: number;
  onPageSizeChange?: (size: number) => void;
  pageSizeOptions?: number[];
}

export function Pagination({
  currentPage,
  totalPages,
  onPageChange,
  pageSize,
  totalItems,
  onPageSizeChange,
  pageSizeOptions = [10, 25, 50, 100],
}: PaginationProps) {
  const pages = Array.from({ length: Math.min(totalPages, 7) }, (_, i) => {
    if (totalPages <= 7) return i + 1;
    if (currentPage <= 4) return i + 1;
    if (currentPage >= totalPages - 3) return totalPages - 6 + i;
    return currentPage - 3 + i;
  });

  return (
    <div className="flex items-center justify-between px-4 py-3 border-t border-dark-700">
      <div className="flex items-center gap-4">
        {totalItems !== undefined && (
          <span className="text-sm text-dark-400">
            Showing {Math.min((currentPage - 1) * (pageSize || 10) + 1, totalItems)} to{' '}
            {Math.min(currentPage * (pageSize || 10), totalItems)} of {totalItems} results
          </span>
        )}
        {onPageSizeChange && (
          <div className="flex items-center gap-2">
            <span className="text-sm text-dark-400">Show</span>
            <select
              value={pageSize}
              onChange={(e) => onPageSizeChange(Number(e.target.value))}
              className="bg-dark-700 border border-dark-600 rounded px-2 py-1 text-sm text-dark-200"
            >
              {pageSizeOptions.map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage === 1}
          className="px-3 py-1 text-sm rounded border border-dark-600 text-dark-300 hover:bg-dark-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Previous
        </button>

        {pages.map((page) => (
          <button
            key={page}
            onClick={() => onPageChange(page)}
            className={`px-3 py-1 text-sm rounded border transition-colors ${
              page === currentPage
                ? 'bg-rusty-500 border-rusty-500 text-white'
                : 'border-dark-600 text-dark-300 hover:bg-dark-700'
            }`}
          >
            {page}
          </button>
        ))}

        <button
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage === totalPages}
          className="px-3 py-1 text-sm rounded border border-dark-600 text-dark-300 hover:bg-dark-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Next
        </button>
      </div>
    </div>
  );
}
