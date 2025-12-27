import { useState, Fragment } from 'react';
import { Dialog, Transition } from '@headlessui/react';
import {
  UserPlusIcon,
  ArrowPathIcon,
  TrashIcon,
  LockClosedIcon,
  CheckIcon,
  XMarkIcon,
  PowerIcon,
} from '@heroicons/react/24/outline';
import { toast } from 'react-hot-toast';
import clsx from 'clsx';
import { UserTable } from '../components/users/UserTable';
import { UserForm } from '../components/users/UserForm';
import {
  useUsers,
  useUserStats,
  useCreateUser,
  useUpdateUser,
  useDeleteUser,
  useToggleUserStatus,
  useResetPassword,
  useUserSessions,
  useTerminateSession,
  useTerminateAllUserSessions,
  useBulkUserAction,
} from '../hooks/useUsers';
import { useRoles } from '../hooks/useRoles';
import type { User, UUID, UserStatus, PaginationParams } from '../types';
import type {
  UserFilters,
  CreateUserRequest,
  UpdateUserRequest,
} from '../services/userService';
import { formatDistanceToNow } from 'date-fns';

// ============================================================================
// User Management Page
// ============================================================================

export function UserManagement() {
  // ============================================================================
  // State
  // ============================================================================

  const [filters, setFilters] = useState<UserFilters>({});
  const [pagination, setPagination] = useState<PaginationParams>({
    page: 1,
    pageSize: 20,
    sortBy: 'createdAt',
    sortOrder: 'desc',
  });
  const [selectedUsers, setSelectedUsers] = useState<UUID[]>([]);
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<User | null>(null);
  const [viewingUser, setViewingUser] = useState<User | null>(null);
  const [sessionsDialogOpen, setSessionsDialogOpen] = useState(false);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [userToDelete, setUserToDelete] = useState<User | null>(null);

  // ============================================================================
  // Queries
  // ============================================================================

  const { data: usersData, isLoading: usersLoading, refetch: refetchUsers } = useUsers(
    filters,
    pagination
  );
  const { data: stats } = useUserStats();
  const { data: roles = [] } = useRoles(false);
  const { data: sessions = [] } = useUserSessions(viewingUser?.id || null);

  // ============================================================================
  // Mutations
  // ============================================================================

  const createUser = useCreateUser();
  const updateUser = useUpdateUser();
  const deleteUser = useDeleteUser();
  const toggleStatus = useToggleUserStatus();
  const resetPassword = useResetPassword();
  const terminateSession = useTerminateSession();
  const terminateAllSessions = useTerminateAllUserSessions();
  const bulkAction = useBulkUserAction();

  // ============================================================================
  // Handlers
  // ============================================================================

  const handleSelectUser = (userId: UUID) => {
    setSelectedUsers((prev) =>
      prev.includes(userId) ? prev.filter((id) => id !== userId) : [...prev, userId]
    );
  };

  const handleSelectAll = (selected: boolean) => {
    if (selected && usersData?.data) {
      setSelectedUsers(usersData.data.map((u) => u.id));
    } else {
      setSelectedUsers([]);
    }
  };

  const handleCreateUser = () => {
    setEditingUser(null);
    setIsFormOpen(true);
  };

  const handleEditUser = (user: User) => {
    setEditingUser(user);
    setIsFormOpen(true);
  };

  const handleFormSubmit = async (data: CreateUserRequest | UpdateUserRequest) => {
    try {
      if (editingUser) {
        await updateUser.mutateAsync({
          id: editingUser.id,
          data: data as UpdateUserRequest,
        });
      } else {
        await createUser.mutateAsync(data as CreateUserRequest);
      }
      setIsFormOpen(false);
      setEditingUser(null);
    } catch (error) {
      // Error handling is done in the mutation hooks
    }
  };

  const handleDeleteUser = (user: User) => {
    setUserToDelete(user);
    setDeleteConfirmOpen(true);
  };

  const confirmDelete = async () => {
    if (!userToDelete) return;

    try {
      await deleteUser.mutateAsync(userToDelete.id);
      setDeleteConfirmOpen(false);
      setUserToDelete(null);
      setSelectedUsers((prev) => prev.filter((id) => id !== userToDelete.id));
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleToggleStatus = async (user: User, status: UserStatus) => {
    try {
      await toggleStatus.mutateAsync({ id: user.id, status });
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleResetPassword = async (user: User) => {
    try {
      await resetPassword.mutateAsync({ userId: user.id });
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleViewSessions = (user: User) => {
    setViewingUser(user);
    setSessionsDialogOpen(true);
  };

  const handleTerminateSession = async (sessionId: UUID) => {
    try {
      await terminateSession.mutateAsync(sessionId);
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleTerminateAllSessions = async () => {
    if (!viewingUser) return;

    try {
      await terminateAllSessions.mutateAsync(viewingUser.id);
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleBulkAction = async (action: 'activate' | 'deactivate' | 'lock' | 'unlock' | 'delete') => {
    if (selectedUsers.length === 0) {
      toast.error('No users selected');
      return;
    }

    try {
      await bulkAction.mutateAsync({ userIds: selectedUsers, action });
      setSelectedUsers([]);
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleFilterChange = (newFilters: Partial<UserFilters>) => {
    setFilters((prev) => ({ ...prev, ...newFilters }));
    setPagination((prev) => ({ ...prev, page: 1 }));
  };

  const handlePageChange = (page: number) => {
    setPagination((prev) => ({ ...prev, page }));
  };

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="border-b border-gray-200 bg-white px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">User Management</h1>
            <p className="mt-1 text-sm text-gray-500">
              Manage users, roles, and permissions
            </p>
          </div>
          <button
            onClick={handleCreateUser}
            className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
          >
            <UserPlusIcon className="h-5 w-5" />
            Create User
          </button>
        </div>

        {/* Stats */}
        {stats && (
          <div className="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-4 lg:grid-cols-7">
            <div className="rounded-lg border border-gray-200 bg-white p-3">
              <div className="text-2xl font-semibold text-gray-900">{stats.total}</div>
              <div className="text-xs text-gray-500">Total Users</div>
            </div>
            <div className="rounded-lg border border-green-200 bg-green-50 p-3">
              <div className="text-2xl font-semibold text-green-900">{stats.active}</div>
              <div className="text-xs text-green-700">Active</div>
            </div>
            <div className="rounded-lg border border-gray-200 bg-gray-50 p-3">
              <div className="text-2xl font-semibold text-gray-900">{stats.inactive}</div>
              <div className="text-xs text-gray-600">Inactive</div>
            </div>
            <div className="rounded-lg border border-red-200 bg-red-50 p-3">
              <div className="text-2xl font-semibold text-red-900">{stats.locked}</div>
              <div className="text-xs text-red-700">Locked</div>
            </div>
            <div className="rounded-lg border border-yellow-200 bg-yellow-50 p-3">
              <div className="text-2xl font-semibold text-yellow-900">{stats.pending}</div>
              <div className="text-xs text-yellow-700">Pending</div>
            </div>
            <div className="rounded-lg border border-blue-200 bg-blue-50 p-3">
              <div className="text-2xl font-semibold text-blue-900">{stats.lastDay}</div>
              <div className="text-xs text-blue-700">Last 24h</div>
            </div>
            <div className="rounded-lg border border-purple-200 bg-purple-50 p-3">
              <div className="text-2xl font-semibold text-purple-900">{stats.lastWeek}</div>
              <div className="text-xs text-purple-700">Last 7d</div>
            </div>
          </div>
        )}
      </div>

      {/* Toolbar */}
      <div className="border-b border-gray-200 bg-white px-6 py-3">
        <div className="flex items-center justify-between gap-4">
          {/* Search and Filters */}
          <div className="flex flex-1 items-center gap-2">
            <input
              type="text"
              placeholder="Search users..."
              value={filters.search || ''}
              onChange={(e) => handleFilterChange({ search: e.target.value })}
              className="block w-64 rounded-md border border-gray-300 px-3 py-2 text-sm placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />

            <select
              value={filters.status || ''}
              onChange={(e) =>
                handleFilterChange({
                  status: e.target.value ? (e.target.value as UserStatus) : undefined,
                })
              }
              className="rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            >
              <option value="">All Status</option>
              <option value="active">Active</option>
              <option value="inactive">Inactive</option>
              <option value="locked">Locked</option>
              <option value="pending">Pending</option>
            </select>

            <select
              value={filters.roleId || ''}
              onChange={(e) =>
                handleFilterChange({ roleId: e.target.value || undefined })
              }
              className="rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            >
              <option value="">All Roles</option>
              {roles.map((role) => (
                <option key={role.id} value={role.id}>
                  {role.name}
                </option>
              ))}
            </select>

            <button
              onClick={() => refetchUsers()}
              className="inline-flex items-center gap-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
            >
              <ArrowPathIcon className="h-4 w-4" />
              Refresh
            </button>
          </div>

          {/* Bulk Actions */}
          {selectedUsers.length > 0 && (
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600">
                {selectedUsers.length} selected
              </span>
              <button
                onClick={() => handleBulkAction('activate')}
                className="inline-flex items-center gap-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
              >
                <CheckIcon className="h-4 w-4" />
                Activate
              </button>
              <button
                onClick={() => handleBulkAction('deactivate')}
                className="inline-flex items-center gap-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
              >
                <XMarkIcon className="h-4 w-4" />
                Deactivate
              </button>
              <button
                onClick={() => handleBulkAction('lock')}
                className="inline-flex items-center gap-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
              >
                <LockClosedIcon className="h-4 w-4" />
                Lock
              </button>
              <button
                onClick={() => handleBulkAction('delete')}
                className="inline-flex items-center gap-1 rounded-md border border-red-300 bg-white px-3 py-2 text-sm font-medium text-red-700 hover:bg-red-50"
              >
                <TrashIcon className="h-4 w-4" />
                Delete
              </button>
            </div>
          )}
        </div>
      </div>

      {/* User Table */}
      <div className="flex-1 overflow-auto p-6">
        <UserTable
          users={usersData?.data || []}
          isLoading={usersLoading}
          selectedUsers={selectedUsers}
          onSelectUser={handleSelectUser}
          onSelectAll={handleSelectAll}
          onEdit={handleEditUser}
          onDelete={handleDeleteUser}
          onToggleStatus={handleToggleStatus}
          onResetPassword={handleResetPassword}
          onViewSessions={handleViewSessions}
          onViewDetails={(user) => {
            setViewingUser(user);
            // Could open a detail panel here
          }}
        />

        {/* Pagination */}
        {usersData && usersData.totalPages > 1 && (
          <div className="mt-4 flex items-center justify-between">
            <div className="text-sm text-gray-700">
              Showing {(pagination.page - 1) * pagination.pageSize + 1} to{' '}
              {Math.min(pagination.page * pagination.pageSize, usersData.total)} of{' '}
              {usersData.total} results
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => handlePageChange(pagination.page - 1)}
                disabled={!usersData.hasPrevious}
                className="rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-50"
              >
                Previous
              </button>
              <button
                onClick={() => handlePageChange(pagination.page + 1)}
                disabled={!usersData.hasNext}
                className="rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-50"
              >
                Next
              </button>
            </div>
          </div>
        )}
      </div>

      {/* User Form Dialog */}
      <UserForm
        isOpen={isFormOpen}
        onClose={() => {
          setIsFormOpen(false);
          setEditingUser(null);
        }}
        onSubmit={handleFormSubmit}
        user={editingUser}
        roles={roles}
        isLoading={createUser.isPending || updateUser.isPending}
      />

      {/* Sessions Dialog */}
      <Transition appear show={sessionsDialogOpen} as={Fragment}>
        <Dialog
          as="div"
          className="relative z-50"
          onClose={() => setSessionsDialogOpen(false)}
        >
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <div className="fixed inset-0 bg-black bg-opacity-25" />
          </Transition.Child>

          <div className="fixed inset-0 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4">
              <Transition.Child
                as={Fragment}
                enter="ease-out duration-300"
                enterFrom="opacity-0 scale-95"
                enterTo="opacity-100 scale-100"
                leave="ease-in duration-200"
                leaveFrom="opacity-100 scale-100"
                leaveTo="opacity-0 scale-95"
              >
                <Dialog.Panel className="w-full max-w-3xl transform overflow-hidden rounded-2xl bg-white p-6 text-left align-middle shadow-xl transition-all">
                  <Dialog.Title className="flex items-center justify-between border-b border-gray-200 pb-4">
                    <div>
                      <h3 className="text-lg font-medium text-gray-900">
                        Active Sessions
                      </h3>
                      <p className="mt-1 text-sm text-gray-500">
                        User: {viewingUser?.username}
                      </p>
                    </div>
                    {sessions.length > 0 && (
                      <button
                        onClick={handleTerminateAllSessions}
                        className="inline-flex items-center gap-2 rounded-md bg-red-600 px-3 py-2 text-sm font-medium text-white hover:bg-red-700"
                      >
                        <PowerIcon className="h-4 w-4" />
                        Terminate All
                      </button>
                    )}
                  </Dialog.Title>

                  <div className="mt-4 space-y-3">
                    {sessions.length === 0 ? (
                      <div className="flex h-32 items-center justify-center text-sm text-gray-500">
                        No active sessions
                      </div>
                    ) : (
                      sessions.map((session) => (
                        <div
                          key={session.id}
                          className="flex items-center justify-between rounded-lg border border-gray-200 p-4"
                        >
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <span className="text-sm font-medium text-gray-900">
                                {session.clientAddress}
                              </span>
                              <span
                                className={clsx(
                                  'inline-flex items-center rounded-md px-2 py-1 text-xs font-medium ring-1 ring-inset',
                                  session.state === 'active'
                                    ? 'bg-green-50 text-green-700 ring-green-600/20'
                                    : 'bg-gray-50 text-gray-600 ring-gray-500/20'
                                )}
                              >
                                {session.state}
                              </span>
                            </div>
                            <div className="mt-1 flex items-center gap-4 text-xs text-gray-500">
                              <span>Database: {session.database}</span>
                              <span>
                                Started:{' '}
                                {formatDistanceToNow(new Date(session.backendStart), {
                                  addSuffix: true,
                                })}
                              </span>
                            </div>
                            {session.currentQuery && (
                              <div className="mt-2 rounded bg-gray-50 p-2 text-xs font-mono text-gray-700">
                                {session.currentQuery.substring(0, 100)}
                                {session.currentQuery.length > 100 && '...'}
                              </div>
                            )}
                          </div>
                          <button
                            onClick={() => handleTerminateSession(session.id)}
                            className="ml-4 inline-flex items-center gap-1 rounded-md border border-red-300 bg-white px-3 py-2 text-sm font-medium text-red-700 hover:bg-red-50"
                          >
                            <PowerIcon className="h-4 w-4" />
                            Terminate
                          </button>
                        </div>
                      ))
                    )}
                  </div>

                  <div className="mt-6 flex justify-end">
                    <button
                      onClick={() => setSessionsDialogOpen(false)}
                      className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                    >
                      Close
                    </button>
                  </div>
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </Dialog>
      </Transition>

      {/* Delete Confirmation Dialog */}
      <Transition appear show={deleteConfirmOpen} as={Fragment}>
        <Dialog
          as="div"
          className="relative z-50"
          onClose={() => setDeleteConfirmOpen(false)}
        >
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <div className="fixed inset-0 bg-black bg-opacity-25" />
          </Transition.Child>

          <div className="fixed inset-0 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4">
              <Transition.Child
                as={Fragment}
                enter="ease-out duration-300"
                enterFrom="opacity-0 scale-95"
                enterTo="opacity-100 scale-100"
                leave="ease-in duration-200"
                leaveFrom="opacity-100 scale-100"
                leaveTo="opacity-0 scale-95"
              >
                <Dialog.Panel className="w-full max-w-md transform overflow-hidden rounded-2xl bg-white p-6 text-left align-middle shadow-xl transition-all">
                  <Dialog.Title className="text-lg font-medium text-gray-900">
                    Delete User
                  </Dialog.Title>
                  <div className="mt-2">
                    <p className="text-sm text-gray-500">
                      Are you sure you want to delete user{' '}
                      <span className="font-medium">{userToDelete?.username}</span>? This
                      action cannot be undone.
                    </p>
                  </div>

                  <div className="mt-4 flex justify-end gap-3">
                    <button
                      onClick={() => setDeleteConfirmOpen(false)}
                      className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={confirmDelete}
                      disabled={deleteUser.isPending}
                      className="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {deleteUser.isPending ? 'Deleting...' : 'Delete User'}
                    </button>
                  </div>
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </Dialog>
      </Transition>
    </div>
  );
}
