import { Fragment, useState, useMemo } from 'react';
import { Menu, Transition } from '@headlessui/react';
import {
  EllipsisVerticalIcon,
  MagnifyingGlassIcon,
  FunnelIcon,
  CheckIcon,
  XMarkIcon,
  LockClosedIcon,
  LockOpenIcon,
  KeyIcon,
  PowerIcon,
  TrashIcon,
  PencilIcon,
  UserCircleIcon,
} from '@heroicons/react/24/outline';
import { formatDistanceToNow } from 'date-fns';
import clsx from 'clsx';
import type { User, UUID, UserStatus } from '../../types';

// ============================================================================
// Types
// ============================================================================

interface UserTableProps {
  users: User[];
  isLoading?: boolean;
  selectedUsers: UUID[];
  onSelectUser: (userId: UUID) => void;
  onSelectAll: (selected: boolean) => void;
  onEdit: (user: User) => void;
  onDelete: (user: User) => void;
  onToggleStatus: (user: User, status: UserStatus) => void;
  onResetPassword: (user: User) => void;
  onViewSessions: (user: User) => void;
  onViewDetails: (user: User) => void;
}

// ============================================================================
// Status Badge Component
// ============================================================================

function StatusBadge({ status }: { status: UserStatus }) {
  const styles = {
    active: 'bg-green-100 text-green-800 ring-green-600/20',
    inactive: 'bg-gray-100 text-gray-600 ring-gray-500/20',
    locked: 'bg-red-100 text-red-700 ring-red-600/20',
    pending: 'bg-yellow-100 text-yellow-800 ring-yellow-600/20',
  };

  const icons = {
    active: CheckIcon,
    inactive: XMarkIcon,
    locked: LockClosedIcon,
    pending: UserCircleIcon,
  };

  const Icon = icons[status];

  return (
    <span
      className={clsx(
        'inline-flex items-center gap-x-1.5 rounded-md px-2 py-1 text-xs font-medium ring-1 ring-inset',
        styles[status]
      )}
    >
      <Icon className="h-3.5 w-3.5" />
      {status.charAt(0).toUpperCase() + status.slice(1)}
    </span>
  );
}

// ============================================================================
// User Table Component
// ============================================================================

export function UserTable({
  users,
  isLoading = false,
  selectedUsers,
  onSelectUser,
  onSelectAll,
  onEdit,
  onDelete,
  onToggleStatus,
  onResetPassword,
  onViewSessions,
  onViewDetails,
}: UserTableProps) {
  const allSelected = users.length > 0 && selectedUsers.length === users.length;
  const someSelected = selectedUsers.length > 0 && !allSelected;

  // ============================================================================
  // Action Menu
  // ============================================================================

  const ActionsMenu = ({ user }: { user: User }) => (
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
                  onClick={() => onViewDetails(user)}
                  className={clsx(
                    active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <UserCircleIcon className="h-4 w-4" />
                  View Details
                </button>
              )}
            </Menu.Item>

            <Menu.Item>
              {({ active }) => (
                <button
                  onClick={() => onEdit(user)}
                  className={clsx(
                    active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <PencilIcon className="h-4 w-4" />
                  Edit User
                </button>
              )}
            </Menu.Item>

            <Menu.Item>
              {({ active }) => (
                <button
                  onClick={() => onViewSessions(user)}
                  className={clsx(
                    active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <PowerIcon className="h-4 w-4" />
                  View Sessions
                </button>
              )}
            </Menu.Item>

            <Menu.Item>
              {({ active }) => (
                <button
                  onClick={() => onResetPassword(user)}
                  className={clsx(
                    active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <KeyIcon className="h-4 w-4" />
                  Reset Password
                </button>
              )}
            </Menu.Item>

            <div className="border-t border-gray-100" />

            {user.status === 'active' ? (
              <Menu.Item>
                {({ active }) => (
                  <button
                    onClick={() => onToggleStatus(user, 'inactive')}
                    className={clsx(
                      active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                      'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                    )}
                  >
                    <XMarkIcon className="h-4 w-4" />
                    Deactivate
                  </button>
                )}
              </Menu.Item>
            ) : user.status === 'inactive' ? (
              <Menu.Item>
                {({ active }) => (
                  <button
                    onClick={() => onToggleStatus(user, 'active')}
                    className={clsx(
                      active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                      'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                    )}
                  >
                    <CheckIcon className="h-4 w-4" />
                    Activate
                  </button>
                )}
              </Menu.Item>
            ) : null}

            {user.status === 'locked' ? (
              <Menu.Item>
                {({ active }) => (
                  <button
                    onClick={() => onToggleStatus(user, 'active')}
                    className={clsx(
                      active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                      'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                    )}
                  >
                    <LockOpenIcon className="h-4 w-4" />
                    Unlock
                  </button>
                )}
              </Menu.Item>
            ) : (
              <Menu.Item>
                {({ active }) => (
                  <button
                    onClick={() => onToggleStatus(user, 'locked')}
                    className={clsx(
                      active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                      'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                    )}
                  >
                    <LockClosedIcon className="h-4 w-4" />
                    Lock
                  </button>
                )}
              </Menu.Item>
            )}

            <div className="border-t border-gray-100" />

            <Menu.Item>
              {({ active }) => (
                <button
                  onClick={() => onDelete(user)}
                  className={clsx(
                    active ? 'bg-red-50 text-red-900' : 'text-red-700',
                    'flex w-full items-center gap-x-2 px-4 py-2 text-sm'
                  )}
                >
                  <TrashIcon className="h-4 w-4" />
                  Delete User
                </button>
              )}
            </Menu.Item>
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
        <div className="text-sm text-gray-500">Loading users...</div>
      </div>
    );
  }

  if (users.length === 0) {
    return (
      <div className="flex h-64 flex-col items-center justify-center gap-2">
        <UserCircleIcon className="h-12 w-12 text-gray-300" />
        <p className="text-sm text-gray-500">No users found</p>
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th scope="col" className="relative w-12 px-6 py-3">
                <input
                  type="checkbox"
                  checked={allSelected}
                  ref={(input) => {
                    if (input) {
                      input.indeterminate = someSelected;
                    }
                  }}
                  onChange={(e) => onSelectAll(e.target.checked)}
                  className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600"
                />
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                User
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Email
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Roles
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Status
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Last Login
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
              >
                Created
              </th>
              <th scope="col" className="relative w-12 px-6 py-3">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 bg-white">
            {users.map((user) => (
              <tr
                key={user.id}
                className={clsx(
                  'hover:bg-gray-50',
                  selectedUsers.includes(user.id) && 'bg-blue-50'
                )}
              >
                <td className="relative w-12 px-6 py-4">
                  <input
                    type="checkbox"
                    checked={selectedUsers.includes(user.id)}
                    onChange={() => onSelectUser(user.id)}
                    className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600"
                  />
                </td>
                <td className="whitespace-nowrap px-6 py-4">
                  <div className="flex items-center gap-3">
                    <div className="h-10 w-10 flex-shrink-0">
                      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-blue-100 text-blue-600">
                        <span className="text-sm font-medium">
                          {user.username.charAt(0).toUpperCase()}
                        </span>
                      </div>
                    </div>
                    <div>
                      <div className="font-medium text-gray-900">{user.username}</div>
                      {user.displayName && (
                        <div className="text-sm text-gray-500">{user.displayName}</div>
                      )}
                    </div>
                  </div>
                </td>
                <td className="whitespace-nowrap px-6 py-4">
                  <div className="text-sm text-gray-900">
                    {user.email || <span className="text-gray-400">-</span>}
                  </div>
                </td>
                <td className="px-6 py-4">
                  <div className="flex flex-wrap gap-1">
                    {user.roles.slice(0, 2).map((role) => (
                      <span
                        key={role.id}
                        className="inline-flex items-center rounded-full bg-blue-50 px-2 py-1 text-xs font-medium text-blue-700 ring-1 ring-inset ring-blue-700/10"
                      >
                        {role.name}
                      </span>
                    ))}
                    {user.roles.length > 2 && (
                      <span className="inline-flex items-center rounded-full bg-gray-50 px-2 py-1 text-xs font-medium text-gray-600">
                        +{user.roles.length - 2}
                      </span>
                    )}
                  </div>
                </td>
                <td className="whitespace-nowrap px-6 py-4">
                  <StatusBadge status={user.status} />
                </td>
                <td className="whitespace-nowrap px-6 py-4 text-sm text-gray-500">
                  {user.lastLogin ? (
                    <span title={new Date(user.lastLogin).toLocaleString()}>
                      {formatDistanceToNow(new Date(user.lastLogin), {
                        addSuffix: true,
                      })}
                    </span>
                  ) : (
                    <span className="text-gray-400">Never</span>
                  )}
                </td>
                <td className="whitespace-nowrap px-6 py-4 text-sm text-gray-500">
                  <span title={new Date(user.createdAt).toLocaleString()}>
                    {formatDistanceToNow(new Date(user.createdAt), {
                      addSuffix: true,
                    })}
                  </span>
                </td>
                <td className="relative whitespace-nowrap px-6 py-4 text-right text-sm font-medium">
                  <ActionsMenu user={user} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
