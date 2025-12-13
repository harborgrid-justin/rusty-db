// ============================================================================
// Node List Component
// Sortable table view of all cluster nodes
// ============================================================================

import { useMemo, useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
} from '@tanstack/react-table';
import {
  CheckCircleIcon,
  ExclamationCircleIcon,
  XCircleIcon,
  ChevronUpIcon,
  ChevronDownIcon,
  EllipsisHorizontalIcon,
} from '@heroicons/react/24/outline';
import { StarIcon } from '@heroicons/react/24/solid';
import { Menu } from '@headlessui/react';
import type { ClusterNode } from '../../types';
import clsx from 'clsx';

interface NodeListProps {
  nodes: ClusterNode[];
  onNodeClick?: (node: ClusterNode) => void;
  onPromote?: (nodeId: string) => void;
  onDemote?: (nodeId: string) => void;
  onRemove?: (nodeId: string) => void;
  onResync?: (nodeId: string) => void;
  selectedNodes?: string[];
  onSelectionChange?: (nodeIds: string[]) => void;
}

const columnHelper = createColumnHelper<ClusterNode>();

export function NodeList({
  nodes,
  onNodeClick,
  onPromote,
  onDemote,
  onRemove,
  onResync,
  selectedNodes = [],
  onSelectionChange,
}: NodeListProps) {
  const [sorting, setSorting] = useState<SortingState>([
    { id: 'role', desc: false },
  ]);

  const columns = useMemo(
    () => [
      // Selection checkbox
      columnHelper.display({
        id: 'select',
        header: ({ table }) => (
          <input
            type="checkbox"
            checked={table.getIsAllRowsSelected()}
            indeterminate={table.getIsSomeRowsSelected()}
            onChange={table.getToggleAllRowsSelectedHandler()}
            className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
          />
        ),
        cell: ({ row }) => (
          <input
            type="checkbox"
            checked={row.getIsSelected()}
            onChange={row.getToggleSelectedHandler()}
            className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
          />
        ),
        size: 40,
      }),

      // Status
      columnHelper.accessor('status', {
        header: 'Status',
        cell: (info) => {
          const status = info.getValue();
          return (
            <div className="flex items-center space-x-2">
              {getStatusIcon(status)}
              <span className="text-sm capitalize">
                {status.replace('_', ' ')}
              </span>
            </div>
          );
        },
        size: 120,
      }),

      // Name
      columnHelper.accessor('name', {
        header: 'Name',
        cell: (info) => {
          const node = info.row.original;
          return (
            <div className="flex items-center space-x-2">
              {node.role === 'leader' && (
                <StarIcon className="w-4 h-4 text-amber-500" />
              )}
              <span className="font-medium text-gray-900">{info.getValue()}</span>
            </div>
          );
        },
        size: 150,
      }),

      // Role
      columnHelper.accessor('role', {
        header: 'Role',
        cell: (info) => {
          const role = info.getValue();
          return (
            <span
              className={clsx(
                'inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium',
                role === 'leader' && 'bg-amber-100 text-amber-800',
                role === 'follower' && 'bg-blue-100 text-blue-800',
                role === 'candidate' && 'bg-purple-100 text-purple-800',
                role === 'observer' && 'bg-gray-100 text-gray-800'
              )}
            >
              {role.toUpperCase()}
            </span>
          );
        },
        size: 100,
      }),

      // Host
      columnHelper.accessor('host', {
        header: 'Host',
        cell: (info) => {
          const node = info.row.original;
          return (
            <span className="text-sm text-gray-600">
              {info.getValue()}:{node.port}
            </span>
          );
        },
        size: 180,
      }),

      // Region/Zone
      columnHelper.display({
        id: 'location',
        header: 'Location',
        cell: ({ row }) => {
          const node = row.original;
          if (!node.region) return <span className="text-sm text-gray-400">-</span>;
          return (
            <span className="text-sm text-gray-600">
              {node.region}
              {node.zone && ` / ${node.zone}`}
            </span>
          );
        },
        size: 150,
      }),

      // CPU
      columnHelper.display({
        id: 'cpu',
        header: 'CPU',
        cell: ({ row }) => {
          const metrics = row.original.metrics;
          if (!metrics) return <span className="text-sm text-gray-400">-</span>;
          return (
            <div className="flex items-center space-x-2">
              <div className="flex-1 h-2 bg-gray-200 rounded-full overflow-hidden max-w-[60px]">
                <div
                  className={clsx(
                    'h-full rounded-full transition-all',
                    metrics.cpu > 80
                      ? 'bg-red-500'
                      : metrics.cpu > 60
                      ? 'bg-amber-500'
                      : 'bg-green-500'
                  )}
                  style={{ width: `${metrics.cpu}%` }}
                />
              </div>
              <span className="text-sm text-gray-600 min-w-[40px]">
                {metrics.cpu.toFixed(0)}%
              </span>
            </div>
          );
        },
        size: 120,
      }),

      // Memory
      columnHelper.display({
        id: 'memory',
        header: 'Memory',
        cell: ({ row }) => {
          const metrics = row.original.metrics;
          if (!metrics) return <span className="text-sm text-gray-400">-</span>;
          return (
            <div className="flex items-center space-x-2">
              <div className="flex-1 h-2 bg-gray-200 rounded-full overflow-hidden max-w-[60px]">
                <div
                  className={clsx(
                    'h-full rounded-full transition-all',
                    metrics.memory > 80
                      ? 'bg-red-500'
                      : metrics.memory > 60
                      ? 'bg-amber-500'
                      : 'bg-green-500'
                  )}
                  style={{ width: `${metrics.memory}%` }}
                />
              </div>
              <span className="text-sm text-gray-600 min-w-[40px]">
                {metrics.memory.toFixed(0)}%
              </span>
            </div>
          );
        },
        size: 120,
      }),

      // Replication Lag
      columnHelper.display({
        id: 'replicationLag',
        header: 'Lag',
        cell: ({ row }) => {
          const node = row.original;
          if (node.role === 'leader' || !node.metrics?.replicationLag) {
            return <span className="text-sm text-gray-400">-</span>;
          }
          const lagSeconds = node.metrics.replicationLag / 1000;
          return (
            <span
              className={clsx(
                'text-sm font-medium',
                lagSeconds > 5
                  ? 'text-red-600'
                  : lagSeconds > 1
                  ? 'text-amber-600'
                  : 'text-green-600'
              )}
            >
              {lagSeconds.toFixed(2)}s
            </span>
          );
        },
        size: 80,
      }),

      // Last Heartbeat
      columnHelper.accessor('lastHeartbeat', {
        header: 'Last Heartbeat',
        cell: (info) => (
          <span className="text-sm text-gray-600">
            {formatDistanceToNow(new Date(info.getValue()), {
              addSuffix: true,
            })}
          </span>
        ),
        size: 140,
      }),

      // Actions
      columnHelper.display({
        id: 'actions',
        header: 'Actions',
        cell: ({ row }) => {
          const node = row.original;
          const isLeader = node.role === 'leader';
          const isHealthy = node.status === 'healthy';

          return (
            <Menu as="div" className="relative inline-block text-left">
              <Menu.Button className="inline-flex items-center justify-center w-8 h-8 rounded-lg hover:bg-gray-100 transition-colors">
                <EllipsisHorizontalIcon className="w-5 h-5 text-gray-600" />
              </Menu.Button>

              <Menu.Items className="absolute right-0 z-10 mt-2 w-48 origin-top-right rounded-lg bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
                <div className="py-1">
                  <Menu.Item>
                    {({ active }) => (
                      <button
                        onClick={() => onNodeClick?.(node)}
                        className={clsx(
                          'w-full px-4 py-2 text-left text-sm',
                          active ? 'bg-gray-100 text-gray-900' : 'text-gray-700'
                        )}
                      >
                        View Details
                      </button>
                    )}
                  </Menu.Item>

                  {!isLeader && onPromote && (
                    <Menu.Item>
                      {({ active }) => (
                        <button
                          onClick={() => onPromote(node.id)}
                          disabled={!isHealthy}
                          className={clsx(
                            'w-full px-4 py-2 text-left text-sm',
                            active && isHealthy
                              ? 'bg-gray-100 text-gray-900'
                              : 'text-gray-700',
                            !isHealthy && 'opacity-50 cursor-not-allowed'
                          )}
                        >
                          Promote to Leader
                        </button>
                      )}
                    </Menu.Item>
                  )}

                  {isLeader && onDemote && (
                    <Menu.Item>
                      {({ active }) => (
                        <button
                          onClick={() => onDemote(node.id)}
                          className={clsx(
                            'w-full px-4 py-2 text-left text-sm',
                            active ? 'bg-gray-100 text-gray-900' : 'text-gray-700'
                          )}
                        >
                          Demote
                        </button>
                      )}
                    </Menu.Item>
                  )}

                  {!isLeader && onResync && (
                    <Menu.Item>
                      {({ active }) => (
                        <button
                          onClick={() => onResync(node.id)}
                          disabled={node.status === 'unreachable'}
                          className={clsx(
                            'w-full px-4 py-2 text-left text-sm',
                            active && node.status !== 'unreachable'
                              ? 'bg-gray-100 text-gray-900'
                              : 'text-gray-700',
                            node.status === 'unreachable' &&
                              'opacity-50 cursor-not-allowed'
                          )}
                        >
                          Resync
                        </button>
                      )}
                    </Menu.Item>
                  )}

                  {!isLeader && onRemove && (
                    <>
                      <div className="border-t border-gray-100 my-1" />
                      <Menu.Item>
                        {({ active }) => (
                          <button
                            onClick={() => onRemove(node.id)}
                            className={clsx(
                              'w-full px-4 py-2 text-left text-sm text-red-600',
                              active && 'bg-red-50'
                            )}
                          >
                            Remove Node
                          </button>
                        )}
                      </Menu.Item>
                    </>
                  )}
                </div>
              </Menu.Items>
            </Menu>
          );
        },
        size: 80,
      }),
    ],
    [onNodeClick, onPromote, onDemote, onRemove, onResync]
  );

  const table = useReactTable({
    data: nodes,
    columns,
    state: {
      sorting,
      rowSelection: selectedNodes.reduce((acc, id) => {
        acc[id] = true;
        return acc;
      }, {} as Record<string, boolean>),
    },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableRowSelection: true,
    onRowSelectionChange: (updater) => {
      if (typeof updater === 'function') {
        const newSelection = updater(
          selectedNodes.reduce((acc, id) => {
            acc[id] = true;
            return acc;
          }, {} as Record<string, boolean>)
        );
        onSelectionChange?.(Object.keys(newSelection));
      }
    },
    getRowId: (row) => row.id,
  });

  function getStatusIcon(status: ClusterNode['status']) {
    switch (status) {
      case 'healthy':
        return <CheckCircleIcon className="w-5 h-5 text-green-500" />;
      case 'degraded':
        return <ExclamationCircleIcon className="w-5 h-5 text-amber-500" />;
      case 'unreachable':
      case 'failed':
        return <XCircleIcon className="w-5 h-5 text-red-500" />;
      default:
        return <div className="w-5 h-5 rounded-full bg-gray-300" />;
    }
  }

  return (
    <div className="overflow-hidden rounded-lg border border-gray-200 bg-white">
      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <th
                    key={header.id}
                    className={clsx(
                      'px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider',
                      header.column.getCanSort() && 'cursor-pointer select-none hover:bg-gray-100'
                    )}
                    onClick={header.column.getToggleSortingHandler()}
                    style={{ width: header.getSize() }}
                  >
                    <div className="flex items-center space-x-1">
                      <span>
                        {flexRender(
                          header.column.columnDef.header,
                          header.getContext()
                        )}
                      </span>
                      {header.column.getCanSort() && (
                        <span className="inline-flex">
                          {header.column.getIsSorted() === 'asc' ? (
                            <ChevronUpIcon className="w-4 h-4" />
                          ) : header.column.getIsSorted() === 'desc' ? (
                            <ChevronDownIcon className="w-4 h-4" />
                          ) : (
                            <div className="w-4 h-4" />
                          )}
                        </span>
                      )}
                    </div>
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody className="bg-white divide-y divide-gray-200">
            {table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                className="hover:bg-gray-50 transition-colors"
              >
                {row.getVisibleCells().map((cell) => (
                  <td
                    key={cell.id}
                    className="px-4 py-4 whitespace-nowrap"
                  >
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {nodes.length === 0 && (
        <div className="text-center py-12">
          <p className="text-gray-500">No nodes found</p>
        </div>
      )}
    </div>
  );
}
