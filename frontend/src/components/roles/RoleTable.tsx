import { Fragment } from 'react';
import { Menu, Transition } from '@headlessui/react';
import {
  EllipsisVerticalIcon,
  ShieldCheckIcon,
  UsersIcon,
  KeyIcon,
  PencilIcon,
  TrashIcon,
  DocumentDuplicateIcon,
} from '@heroicons/react/24/outline';
import { formatDistanceToNow } from 'date-fns';
import clsx from 'clsx';
import type { RoleWithStats } from '../../services/roleService';

// ============================================================================
// Props
// ============================================================================

interface RoleTableProps {
  roles: RoleWithStats[];
  isLoading?: boolean;
  onEdit: (role: RoleWithStats) => void;
  onDelete: (role: RoleWithStats) => void;
  onClone: (role: RoleWithStats) => void;
  onViewUsers: (role: RoleWithStats) => void;
  onViewPermissions: (role: RoleWithStats) => void;
}

// ============================================================================
// Role Table Component
// ============================================================================

export function RoleTable({
  roles,
  isLoading = false,
  onEdit,
  onDelete,
  onClone,
  onViewUsers,
  onViewPermissions,
}: RoleTableProps) {
  // ============================================================================
  // Action Menu
  // ============================================================================

  const ActionsMenu = ({ role }: { role: RoleWithStats }) => (
    <Menu as="div" className="relative inline-block text-left">
      <Menu.Button className="flex items-center rounded-full text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2">
        <span className="sr-only">Open options</span>
        <EllipsisVerticalIcon className="h-5 w-5" />
      </Menu.Button>

      <Transition
        as={Fragment}
        enter="transition ease-out duration-100"
        enterFrom="transform opacity-0 scale-95"
        enterTo="transform opacity-100 scale-100"
        leave="transition ease-in duration-75"
        leaveFrom="transform opacity-100 scale-100"
        leaveTo="transform opacity-0 scale-95"
      >
        <Menu.Items className="absolute right-0 z-10 mt-2 w-48 origin-top-right rounded-md bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
          <div className="py-1">
            <Menu.Item>
              {({ active }) => (
                <button
                  onClick={() => onViewPermissions(role)}
                  className={clsx(
                    active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <KeyIcon className="h-4 w-4" />
                  View Permissions
                </button>
              )}
            </Menu.Item>

            <Menu.Item>
              {({ active }) => (
                <button
                  onClick={() => onViewUsers(role)}
                  className={clsx(
                    active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <UsersIcon className="h-4 w-4" />
                  View Users ({role.userCount})
                </button>
              )}
            </Menu.Item>

            {!role.isSystem && (
              <>
                <div className="border-t border-gray-100" />

                <Menu.Item>
                  {({ active }) => (
                    <button
                      onClick={() => onEdit(role)}
                      className={clsx(
                        active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                        'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                      )}
                    >
                      <PencilIcon className="h-4 w-4" />
                      Edit Role
                    </button>
                  )}
                </Menu.Item>

                <Menu.Item>
                  {({ active }) => (
                    <button
                      onClick={() => onClone(role)}
                      className={clsx(
                        active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                        'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                      )}
                    >
                      <DocumentDuplicateIcon className="h-4 w-4" />
                      Clone Role
                    </button>
                  )}
                </Menu.Item>

                <div className="border-t border-gray-100" />

                <Menu.Item>
                  {({ active }) => (
                    <button
                      onClick={() => onDelete(role)}
                      disabled={role.userCount > 0}
                      className={clsx(
                        active && role.userCount === 0
                          ? 'bg-red-50 text-red-900'
                          : 'text-red-700',
                        role.userCount > 0 && 'cursor-not-allowed opacity-50',
                        'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                      )}
                    >
                      <TrashIcon className="h-4 w-4" />
                      Delete Role
                    </button>
                  )}
                </Menu.Item>
              </>
            )}
          </div>
        </Menu.Items>
      </Transition>
    </Menu>
  );

  // ============================================================================
  // Render
  // ============================================================================

  if (isLoading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <div className="text-sm text-gray-500">Loading roles...</div>
      </div>
    );
  }

  if (roles.length === 0) {
    return (
      <div className="flex h-64 flex-col items-center justify-center gap-2">
        <ShieldCheckIcon className="h-12 w-12 text-gray-300" />
        <p className="text-sm text-gray-500">No roles found</p>
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Role
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Description
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Permissions
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Users
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Created
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Last Updated
              </th>
              <th scope="col" className="relative w-12 px-6 py-3">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 bg-white">
            {roles.map((role) => (
              <tr key={role.id} className="hover:bg-gray-50">
                <td className="whitespace-nowrap px-6 py-4">
                  <div className="flex items-center gap-3">
                    <div
                      className={clsx(
                        'flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg',
                        role.isSystem
                          ? 'bg-purple-100 text-purple-600'
                          : 'bg-blue-100 text-blue-600'
                      )}
                    >
                      <ShieldCheckIcon className="h-5 w-5" />
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-gray-900">{role.name}</span>
                        {role.isSystem && (
                          <span className="inline-flex items-center rounded-md bg-purple-50 px-2 py-1 text-xs font-medium text-purple-700 ring-1 ring-inset ring-purple-700/10">
                            System Role
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                </td>
                <td className="px-6 py-4">
                  <div className="max-w-xs text-sm text-gray-900">
                    {role.description || (
                      <span className="text-gray-400">No description</span>
                    )}
                  </div>
                </td>
                <td className="whitespace-nowrap px-6 py-4">
                  <button
                    onClick={() => onViewPermissions(role)}
                    className="group flex items-center gap-2 text-sm font-medium text-blue-600 hover:text-blue-700"
                  >
                    <KeyIcon className="h-4 w-4" />
                    <span className="group-hover:underline">
                      {role.permissionCount} permission{role.permissionCount !== 1 ? 's' : ''}
                    </span>
                  </button>
                </td>
                <td className="whitespace-nowrap px-6 py-4">
                  <button
                    onClick={() => onViewUsers(role)}
                    className="group flex items-center gap-2 text-sm font-medium text-gray-900 hover:text-blue-600"
                  >
                    <UsersIcon className="h-4 w-4" />
                    <span className="group-hover:underline">
                      {role.userCount} user{role.userCount !== 1 ? 's' : ''}
                    </span>
                  </button>
                </td>
                <td className="whitespace-nowrap px-6 py-4 text-sm text-gray-500">
                  <span title={new Date(role.createdAt).toLocaleString()}>
                    {formatDistanceToNow(new Date(role.createdAt), {
                      addSuffix: true,
                    })}
                  </span>
                </td>
                <td className="whitespace-nowrap px-6 py-4 text-sm text-gray-500">
                  <span title={new Date(role.updatedAt).toLocaleString()}>
                    {formatDistanceToNow(new Date(role.updatedAt), {
                      addSuffix: true,
                    })}
                  </span>
                </td>
                <td className="relative whitespace-nowrap px-6 py-4 text-right text-sm font-medium">
                  <ActionsMenu role={role} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
