import { useState, useMemo } from 'react';
import {
  MagnifyingGlassIcon,
  CheckIcon,
  XMarkIcon,
  InformationCircleIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { Permission, PermissionAction, UUID } from '../../types';

// ============================================================================
// Types
// ============================================================================

interface PermissionMatrixProps {
  permissions: Permission[];
  selectedPermissionIds: UUID[];
  onChange: (permissionIds: UUID[]) => void;
  disabled?: boolean;
  className?: string;
}

interface MatrixCell {
  permission: Permission | null;
  resource: string;
  action: PermissionAction;
}

// ============================================================================
// Helper Functions
// ============================================================================

function buildMatrix(permissions: Permission[]): {
  resources: string[];
  actions: PermissionAction[];
  matrix: Record<string, Record<PermissionAction, Permission | null>>;
} {
  const resourceSet = new Set<string>();
  const actionSet = new Set<PermissionAction>();
  const matrix: Record<string, Record<PermissionAction, Permission | null>> = {};

  // Collect all resources and actions
  permissions.forEach((perm) => {
    resourceSet.add(perm.resource);
    actionSet.add(perm.action);
  });

  const resources = Array.from(resourceSet).sort();
  const actions: PermissionAction[] = ['read', 'create', 'update', 'delete', 'execute', 'grant', 'admin'];

  // Build matrix
  resources.forEach((resource) => {
    matrix[resource] = {} as Record<PermissionAction, Permission | null>;
    actions.forEach((action) => {
      const permission = permissions.find(
        (p) => p.resource === resource && p.action === action
      );
      matrix[resource][action] = permission || null;
    });
  });

  return { resources, actions, matrix };
}

// ============================================================================
// Action Badge Component
// ============================================================================

function ActionBadge({ action }: { action: PermissionAction }) {
  const colors: Record<PermissionAction, string> = {
    read: 'bg-blue-100 text-blue-700 ring-blue-600/20',
    create: 'bg-green-100 text-green-700 ring-green-600/20',
    update: 'bg-yellow-100 text-yellow-700 ring-yellow-600/20',
    delete: 'bg-red-100 text-red-700 ring-red-600/20',
    execute: 'bg-purple-100 text-purple-700 ring-purple-600/20',
    grant: 'bg-orange-100 text-orange-700 ring-orange-600/20',
    admin: 'bg-gray-900 text-white ring-gray-600/20',
  };

  return (
    <span
      className={clsx(
        'inline-flex items-center rounded-md px-2 py-1 text-xs font-medium ring-1 ring-inset',
        colors[action]
      )}
    >
      {action.charAt(0).toUpperCase() + action.slice(1)}
    </span>
  );
}

// ============================================================================
// Permission Matrix Component
// ============================================================================

export function PermissionMatrix({
  permissions,
  selectedPermissionIds,
  onChange,
  disabled = false,
  className,
}: PermissionMatrixProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [hoveredCell, setHoveredCell] = useState<{
    resource: string;
    action: PermissionAction;
  } | null>(null);

  // Build matrix structure
  const { resources, actions, matrix } = useMemo(
    () => buildMatrix(permissions),
    [permissions]
  );

  // Filter resources by search
  const filteredResources = useMemo(() => {
    if (!searchQuery) return resources;
    const query = searchQuery.toLowerCase();
    return resources.filter((resource) => resource.toLowerCase().includes(query));
  }, [resources, searchQuery]);

  // Check if permission is selected
  const isPermissionSelected = (permission: Permission | null): boolean => {
    if (!permission) return false;
    return selectedPermissionIds.includes(permission.id);
  };

  // Toggle single permission
  const togglePermission = (permission: Permission | null) => {
    if (!permission || disabled) return;

    if (selectedPermissionIds.includes(permission.id)) {
      onChange(selectedPermissionIds.filter((id) => id !== permission.id));
    } else {
      onChange([...selectedPermissionIds, permission.id]);
    }
  };

  // Select all permissions for a resource (row)
  const toggleRow = (resource: string) => {
    if (disabled) return;

    const rowPermissions = actions
      .map((action) => matrix[resource]?.[action])
      .filter((p): p is Permission => p !== null);

    const allSelected = rowPermissions.every((p) =>
      selectedPermissionIds.includes(p.id)
    );

    if (allSelected) {
      // Deselect all in row
      const rowIds = rowPermissions.map((p) => p.id);
      onChange(selectedPermissionIds.filter((id) => !rowIds.includes(id)));
    } else {
      // Select all in row
      const newIds = rowPermissions.map((p) => p.id);
      const combined = Array.from(new Set([...selectedPermissionIds, ...newIds]));
      onChange(combined);
    }
  };

  // Select all permissions for an action (column)
  const toggleColumn = (action: PermissionAction) => {
    if (disabled) return;

    const colPermissions = resources
      .map((resource) => matrix[resource]?.[action])
      .filter((p): p is Permission => p !== null);

    const allSelected = colPermissions.every((p) =>
      selectedPermissionIds.includes(p.id)
    );

    if (allSelected) {
      // Deselect all in column
      const colIds = colPermissions.map((p) => p.id);
      onChange(selectedPermissionIds.filter((id) => !colIds.includes(id)));
    } else {
      // Select all in column
      const newIds = colPermissions.map((p) => p.id);
      const combined = Array.from(new Set([...selectedPermissionIds, ...newIds]));
      onChange(combined);
    }
  };

  // Select all permissions
  const toggleAll = () => {
    if (disabled) return;

    if (selectedPermissionIds.length === permissions.length) {
      onChange([]);
    } else {
      onChange(permissions.map((p) => p.id));
    }
  };

  const allSelected = selectedPermissionIds.length === permissions.length;

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <div className={clsx('space-y-4', className)}>
      {/* Header with search and info */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex-1">
          <div className="relative">
            <div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
              <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
            </div>
            <input
              type="text"
              placeholder="Search resources..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="block w-full rounded-md border border-gray-300 py-2 pl-10 pr-3 text-sm placeholder-gray-500 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
        </div>

        <div className="flex items-center gap-2 text-sm text-gray-600">
          <InformationCircleIcon className="h-5 w-5 text-gray-400" />
          <span>
            {selectedPermissionIds.length} of {permissions.length} selected
          </span>
        </div>
      </div>

      {/* Matrix Table */}
      <div className="overflow-x-auto rounded-lg border border-gray-200 bg-white shadow-sm">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th
                scope="col"
                className="sticky left-0 z-10 bg-gray-50 px-6 py-3 text-left"
              >
                <button
                  type="button"
                  onClick={toggleAll}
                  disabled={disabled}
                  className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-gray-500 hover:text-gray-700 disabled:cursor-not-allowed"
                >
                  <input
                    type="checkbox"
                    checked={allSelected}
                    onChange={() => {}}
                    disabled={disabled}
                    className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600 disabled:cursor-not-allowed disabled:opacity-50"
                  />
                  Resource
                </button>
              </th>
              {actions.map((action) => {
                const colPermissions = resources
                  .map((resource) => matrix[resource]?.[action])
                  .filter((p): p is Permission => p !== null);
                const colAllSelected = colPermissions.every((p) =>
                  selectedPermissionIds.includes(p.id)
                );

                return (
                  <th
                    key={action}
                    scope="col"
                    className="px-3 py-3 text-center"
                  >
                    <button
                      type="button"
                      onClick={() => toggleColumn(action)}
                      disabled={disabled}
                      className="flex w-full flex-col items-center gap-1 disabled:cursor-not-allowed"
                    >
                      <input
                        type="checkbox"
                        checked={colAllSelected}
                        onChange={() => {}}
                        disabled={disabled}
                        className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600 disabled:cursor-not-allowed disabled:opacity-50"
                      />
                      <ActionBadge action={action} />
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 bg-white">
            {filteredResources.length === 0 ? (
              <tr>
                <td
                  colSpan={actions.length + 1}
                  className="px-6 py-8 text-center text-sm text-gray-500"
                >
                  No resources found matching "{searchQuery}"
                </td>
              </tr>
            ) : (
              filteredResources.map((resource) => {
                const rowPermissions = actions
                  .map((action) => matrix[resource]?.[action])
                  .filter((p): p is Permission => p !== null);
                const rowAllSelected = rowPermissions.every((p) =>
                  selectedPermissionIds.includes(p.id)
                );

                return (
                  <tr key={resource} className="hover:bg-gray-50">
                    <td className="sticky left-0 z-10 bg-white px-6 py-4 hover:bg-gray-50">
                      <button
                        type="button"
                        onClick={() => toggleRow(resource)}
                        disabled={disabled}
                        className="flex items-center gap-3 text-left disabled:cursor-not-allowed"
                      >
                        <input
                          type="checkbox"
                          checked={rowAllSelected}
                          onChange={() => {}}
                          disabled={disabled}
                          className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600 disabled:cursor-not-allowed disabled:opacity-50"
                        />
                        <div>
                          <div className="text-sm font-medium text-gray-900">
                            {resource}
                          </div>
                          <div className="text-xs text-gray-500">
                            {rowPermissions.length} permission
                            {rowPermissions.length !== 1 ? 's' : ''}
                          </div>
                        </div>
                      </button>
                    </td>
                    {actions.map((action) => {
                      const permission = matrix[resource]?.[action];
                      const isSelected = isPermissionSelected(permission);
                      const isHovered =
                        hoveredCell?.resource === resource &&
                        hoveredCell?.action === action;

                      return (
                        <td
                          key={action}
                          className="px-3 py-4 text-center"
                          onMouseEnter={() =>
                            setHoveredCell({ resource, action })
                          }
                          onMouseLeave={() => setHoveredCell(null)}
                        >
                          {permission ? (
                            <button
                              type="button"
                              onClick={() => togglePermission(permission)}
                              disabled={disabled}
                              title={permission.description || permission.name}
                              className={clsx(
                                'inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors',
                                isSelected
                                  ? 'bg-blue-600 text-white hover:bg-blue-700'
                                  : 'bg-gray-100 text-gray-400 hover:bg-gray-200 hover:text-gray-600',
                                isHovered && 'ring-2 ring-blue-500 ring-offset-1',
                                disabled && 'cursor-not-allowed opacity-50'
                              )}
                            >
                              {isSelected ? (
                                <CheckIcon className="h-5 w-5" />
                              ) : (
                                <XMarkIcon className="h-5 w-5" />
                              )}
                            </button>
                          ) : (
                            <div className="inline-flex h-8 w-8 items-center justify-center">
                              <span className="text-gray-300">-</span>
                            </div>
                          )}
                        </td>
                      );
                    })}
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* Footer with legend */}
      <div className="rounded-md bg-gray-50 p-4">
        <div className="flex items-start gap-2">
          <InformationCircleIcon className="h-5 w-5 flex-shrink-0 text-gray-400" />
          <div className="flex-1 text-xs text-gray-600">
            <p className="font-medium">Permission Matrix Guide:</p>
            <ul className="mt-1 list-inside list-disc space-y-0.5">
              <li>Click individual cells to toggle specific permissions</li>
              <li>Click row checkboxes to select all permissions for a resource</li>
              <li>Click column checkboxes to select all permissions for an action</li>
              <li>
                Hover over cells to see permission details (name and description)
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
