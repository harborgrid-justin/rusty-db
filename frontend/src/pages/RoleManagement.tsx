import { useState, Fragment, useEffect } from 'react';
import { Dialog, Transition } from '@headlessui/react';
import {
  ShieldCheckIcon,
  PlusIcon,
  ArrowPathIcon,
  XMarkIcon,
  UsersIcon,
  KeyIcon,
  DocumentDuplicateIcon,
} from '@heroicons/react/24/outline';
import { useForm, Controller } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { toast } from 'react-hot-toast';
import clsx from 'clsx';
import { RoleTable } from '../components/roles/RoleTable';
import { PermissionMatrix } from '../components/roles/PermissionMatrix';
import {
  useRoles,
  useCreateRole,
  useUpdateRole,
  useDeleteRole,
  useCloneRole,
  useRoleUsers,
  usePermissions,
} from '../hooks/useRoles';
import type { UUID, Role } from '../types';
import type { CreateRoleRequest, UpdateRoleRequest, RoleWithStats } from '../services/roleService';

// ============================================================================
// Validation Schema
// ============================================================================

const roleSchema = z.object({
  name: z
    .string()
    .min(2, 'Role name must be at least 2 characters')
    .max(50, 'Role name must be less than 50 characters')
    .regex(/^[a-zA-Z0-9_-\s]+$/, 'Role name can only contain letters, numbers, spaces, hyphens, and underscores'),
  description: z
    .string()
    .max(500, 'Description must be less than 500 characters')
    .optional()
    .or(z.literal('')),
  permissionIds: z.array(z.string()).min(1, 'At least one permission must be selected'),
});

type RoleFormData = z.infer<typeof roleSchema>;

// ============================================================================
// Role Form Dialog
// ============================================================================

interface RoleFormDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: CreateRoleRequest | UpdateRoleRequest) => void;
  role?: RoleWithStats | null;
  permissions: any[];
  isLoading?: boolean;
}

function RoleFormDialog({
  isOpen,
  onClose,
  onSubmit,
  role,
  permissions,
  isLoading = false,
}: RoleFormDialogProps) {
  const isEditMode = !!role;

  const {
    register,
    handleSubmit,
    control,
    reset,
    formState: { errors },
  } = useForm<RoleFormData>({
    resolver: zodResolver(roleSchema),
    defaultValues: isEditMode
      ? {
          name: role.name,
          description: role.description || '',
          permissionIds: role.permissions.map((p) => p.id),
        }
      : {
          name: '',
          description: '',
          permissionIds: [],
        },
  });

  useEffect(() => {
    if (isOpen) {
      if (isEditMode && role) {
        reset({
          name: role.name,
          description: role.description || '',
          permissionIds: role.permissions.map((p) => p.id),
        });
      } else {
        reset({
          name: '',
          description: '',
          permissionIds: [],
        });
      }
    }
  }, [isOpen, isEditMode, role, reset]);

  const handleFormSubmit = (data: RoleFormData) => {
    onSubmit({
      name: data.name,
      description: data.description || undefined,
      permissionIds: data.permissionIds,
    });
  };

  return (
    <Transition appear show={isOpen} as={Fragment}>
      <Dialog as="div" className="relative z-50" onClose={onClose}>
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
              <Dialog.Panel className="w-full max-w-5xl transform overflow-hidden rounded-2xl bg-white p-6 text-left align-middle shadow-xl transition-all">
                <Dialog.Title className="flex items-center justify-between border-b border-gray-200 pb-4">
                  <h3 className="text-lg font-medium text-gray-900">
                    {isEditMode ? `Edit Role: ${role.name}` : 'Create New Role'}
                  </h3>
                  <button
                    type="button"
                    onClick={onClose}
                    className="rounded-md text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    <XMarkIcon className="h-6 w-6" />
                  </button>
                </Dialog.Title>

                <form onSubmit={handleSubmit(handleFormSubmit)} className="mt-6 space-y-6">
                  {/* Basic Info */}
                  <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
                    <div>
                      <label htmlFor="name" className="block text-sm font-medium text-gray-700">
                        Role Name <span className="text-red-500">*</span>
                      </label>
                      <input
                        {...register('name')}
                        type="text"
                        disabled={isEditMode && role?.isSystem}
                        className={clsx(
                          'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                          errors.name
                            ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                            : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500',
                          isEditMode && role?.isSystem && 'cursor-not-allowed bg-gray-100'
                        )}
                        placeholder="Database Administrator"
                      />
                      {errors.name && (
                        <p className="mt-1 text-sm text-red-600">{errors.name.message}</p>
                      )}
                    </div>

                    <div>
                      <label htmlFor="description" className="block text-sm font-medium text-gray-700">
                        Description
                      </label>
                      <input
                        {...register('description')}
                        type="text"
                        className={clsx(
                          'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                          errors.description
                            ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                            : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                        )}
                        placeholder="Full access to all database operations"
                      />
                      {errors.description && (
                        <p className="mt-1 text-sm text-red-600">{errors.description.message}</p>
                      )}
                    </div>
                  </div>

                  {/* Permission Matrix */}
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      Permissions <span className="text-red-500">*</span>
                    </label>
                    <Controller
                      name="permissionIds"
                      control={control}
                      render={({ field }) => (
                        <PermissionMatrix
                          permissions={permissions}
                          selectedPermissionIds={field.value}
                          onChange={field.onChange}
                          disabled={isEditMode && role?.isSystem}
                        />
                      )}
                    />
                    {errors.permissionIds && (
                      <p className="mt-2 text-sm text-red-600">{errors.permissionIds.message}</p>
                    )}
                  </div>

                  {/* System Role Warning */}
                  {isEditMode && role?.isSystem && (
                    <div className="rounded-md bg-yellow-50 p-4">
                      <div className="flex">
                        <div className="flex-shrink-0">
                          <ShieldCheckIcon className="h-5 w-5 text-yellow-400" />
                        </div>
                        <div className="ml-3">
                          <h3 className="text-sm font-medium text-yellow-800">System Role</h3>
                          <div className="mt-2 text-sm text-yellow-700">
                            <p>
                              This is a system role. You can only modify the description and permissions,
                              but not the name.
                            </p>
                          </div>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Actions */}
                  <div className="flex justify-end gap-3 border-t border-gray-200 pt-4">
                    <button
                      type="button"
                      onClick={onClose}
                      className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      disabled={isLoading || (isEditMode && role?.isSystem)}
                      className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {isLoading ? 'Saving...' : isEditMode ? 'Update Role' : 'Create Role'}
                    </button>
                  </div>
                </form>
              </Dialog.Panel>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition>
  );
}

// ============================================================================
// Role Users Dialog
// ============================================================================

interface RoleUsersDialogProps {
  isOpen: boolean;
  onClose: () => void;
  role: RoleWithStats | null;
  users: any[];
}

function RoleUsersDialog({ isOpen, onClose, role, users }: RoleUsersDialogProps) {
  return (
    <Transition appear show={isOpen} as={Fragment}>
      <Dialog as="div" className="relative z-50" onClose={onClose}>
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
              <Dialog.Panel className="w-full max-w-2xl transform overflow-hidden rounded-2xl bg-white p-6 text-left align-middle shadow-xl transition-all">
                <Dialog.Title className="border-b border-gray-200 pb-4">
                  <h3 className="text-lg font-medium text-gray-900">
                    Users with Role: {role?.name}
                  </h3>
                  <p className="mt-1 text-sm text-gray-500">
                    {users.length} user{users.length !== 1 ? 's' : ''} assigned to this role
                  </p>
                </Dialog.Title>

                <div className="mt-4 max-h-96 overflow-y-auto">
                  {users.length === 0 ? (
                    <div className="flex h-32 items-center justify-center text-sm text-gray-500">
                      No users assigned to this role
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {users.map((user) => (
                        <div
                          key={user.id}
                          className="flex items-center justify-between rounded-lg border border-gray-200 p-3"
                        >
                          <div className="flex items-center gap-3">
                            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-blue-100 text-blue-600">
                              <span className="text-sm font-medium">
                                {user.username.charAt(0).toUpperCase()}
                              </span>
                            </div>
                            <div>
                              <div className="font-medium text-gray-900">{user.username}</div>
                              {user.displayName && (
                                <div className="text-sm text-gray-500">{user.displayName}</div>
                              )}
                            </div>
                          </div>
                          {user.email && (
                            <div className="text-sm text-gray-500">{user.email}</div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <div className="mt-6 flex justify-end">
                  <button
                    onClick={onClose}
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
  );
}

// ============================================================================
// Role Management Page
// ============================================================================

export function RoleManagement() {
  // ============================================================================
  // State
  // ============================================================================

  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<RoleWithStats | null>(null);
  const [viewingRoleUsers, setViewingRoleUsers] = useState<RoleWithStats | null>(null);
  const [usersDialogOpen, setUsersDialogOpen] = useState(false);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [roleToDelete, setRoleToDelete] = useState<RoleWithStats | null>(null);
  const [cloneDialogOpen, setCloneDialogOpen] = useState(false);
  const [roleToClone, setRoleToClone] = useState<RoleWithStats | null>(null);
  const [cloneName, setCloneName] = useState('');

  // ============================================================================
  // Queries
  // ============================================================================

  const { data: roles = [], isLoading: rolesLoading, refetch: refetchRoles } = useRoles(true);
  const { data: permissions = [] } = usePermissions();
  const { data: roleUsers = [] } = useRoleUsers(viewingRoleUsers?.id || null);

  // ============================================================================
  // Mutations
  // ============================================================================

  const createRole = useCreateRole();
  const updateRole = useUpdateRole();
  const deleteRole = useDeleteRole();
  const cloneRole = useCloneRole();

  // ============================================================================
  // Handlers
  // ============================================================================

  const handleCreateRole = () => {
    setEditingRole(null);
    setIsFormOpen(true);
  };

  const handleEditRole = (role: RoleWithStats) => {
    if (role.isSystem) {
      toast.error('System roles cannot be edited');
      return;
    }
    setEditingRole(role);
    setIsFormOpen(true);
  };

  const handleFormSubmit = async (data: CreateRoleRequest | UpdateRoleRequest) => {
    try {
      if (editingRole) {
        await updateRole.mutateAsync({
          id: editingRole.id,
          data: data as UpdateRoleRequest,
        });
      } else {
        await createRole.mutateAsync(data as CreateRoleRequest);
      }
      setIsFormOpen(false);
      setEditingRole(null);
    } catch (error) {
      // Error handling is done in the mutation hooks
    }
  };

  const handleDeleteRole = (role: RoleWithStats) => {
    if (role.isSystem) {
      toast.error('System roles cannot be deleted');
      return;
    }
    if (role.userCount > 0) {
      toast.error(`Cannot delete role with ${role.userCount} assigned users`);
      return;
    }
    setRoleToDelete(role);
    setDeleteConfirmOpen(true);
  };

  const confirmDelete = async () => {
    if (!roleToDelete) return;

    try {
      await deleteRole.mutateAsync(roleToDelete.id);
      setDeleteConfirmOpen(false);
      setRoleToDelete(null);
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleCloneRole = (role: RoleWithStats) => {
    setRoleToClone(role);
    setCloneName(`${role.name} (Copy)`);
    setCloneDialogOpen(true);
  };

  const confirmClone = async () => {
    if (!roleToClone || !cloneName.trim()) {
      toast.error('Please provide a name for the cloned role');
      return;
    }

    try {
      await cloneRole.mutateAsync({ roleId: roleToClone.id, newName: cloneName });
      setCloneDialogOpen(false);
      setRoleToClone(null);
      setCloneName('');
    } catch (error) {
      // Error handling is done in the mutation hook
    }
  };

  const handleViewUsers = (role: RoleWithStats) => {
    setViewingRoleUsers(role);
    setUsersDialogOpen(true);
  };

  const handleViewPermissions = (role: RoleWithStats) => {
    toast.info(`Viewing permissions for ${role.name}`);
    // Could open a detail panel or modal here
  };

  // ============================================================================
  // Stats
  // ============================================================================

  const stats = {
    total: roles.length,
    system: roles.filter((r) => r.isSystem).length,
    custom: roles.filter((r) => !r.isSystem).length,
    totalUsers: roles.reduce((sum, r) => sum + r.userCount, 0),
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
            <h1 className="text-2xl font-bold text-gray-900">Role Management</h1>
            <p className="mt-1 text-sm text-gray-500">
              Manage roles and permissions for access control
            </p>
          </div>
          <button
            onClick={handleCreateRole}
            className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
          >
            <PlusIcon className="h-5 w-5" />
            Create Role
          </button>
        </div>

        {/* Stats */}
        <div className="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-4">
          <div className="rounded-lg border border-gray-200 bg-white p-3">
            <div className="text-2xl font-semibold text-gray-900">{stats.total}</div>
            <div className="text-xs text-gray-500">Total Roles</div>
          </div>
          <div className="rounded-lg border border-purple-200 bg-purple-50 p-3">
            <div className="text-2xl font-semibold text-purple-900">{stats.system}</div>
            <div className="text-xs text-purple-700">System Roles</div>
          </div>
          <div className="rounded-lg border border-blue-200 bg-blue-50 p-3">
            <div className="text-2xl font-semibold text-blue-900">{stats.custom}</div>
            <div className="text-xs text-blue-700">Custom Roles</div>
          </div>
          <div className="rounded-lg border border-green-200 bg-green-50 p-3">
            <div className="text-2xl font-semibold text-green-900">{stats.totalUsers}</div>
            <div className="text-xs text-green-700">Assigned Users</div>
          </div>
        </div>
      </div>

      {/* Toolbar */}
      <div className="border-b border-gray-200 bg-white px-6 py-3">
        <div className="flex items-center justify-between">
          <div className="text-sm text-gray-600">
            Showing {roles.length} role{roles.length !== 1 ? 's' : ''}
          </div>
          <button
            onClick={() => refetchRoles()}
            className="inline-flex items-center gap-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
          >
            <ArrowPathIcon className="h-4 w-4" />
            Refresh
          </button>
        </div>
      </div>

      {/* Role Table */}
      <div className="flex-1 overflow-auto p-6">
        <RoleTable
          roles={roles}
          isLoading={rolesLoading}
          onEdit={handleEditRole}
          onDelete={handleDeleteRole}
          onClone={handleCloneRole}
          onViewUsers={handleViewUsers}
          onViewPermissions={handleViewPermissions}
        />
      </div>

      {/* Role Form Dialog */}
      <RoleFormDialog
        isOpen={isFormOpen}
        onClose={() => {
          setIsFormOpen(false);
          setEditingRole(null);
        }}
        onSubmit={handleFormSubmit}
        role={editingRole}
        permissions={permissions}
        isLoading={createRole.isPending || updateRole.isPending}
      />

      {/* Role Users Dialog */}
      <RoleUsersDialog
        isOpen={usersDialogOpen}
        onClose={() => {
          setUsersDialogOpen(false);
          setViewingRoleUsers(null);
        }}
        role={viewingRoleUsers}
        users={roleUsers}
      />

      {/* Delete Confirmation Dialog */}
      <Transition appear show={deleteConfirmOpen} as={Fragment}>
        <Dialog as="div" className="relative z-50" onClose={() => setDeleteConfirmOpen(false)}>
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
                    Delete Role
                  </Dialog.Title>
                  <div className="mt-2">
                    <p className="text-sm text-gray-500">
                      Are you sure you want to delete role{' '}
                      <span className="font-medium">{roleToDelete?.name}</span>? This action
                      cannot be undone.
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
                      disabled={deleteRole.isPending}
                      className="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {deleteRole.isPending ? 'Deleting...' : 'Delete Role'}
                    </button>
                  </div>
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </Dialog>
      </Transition>

      {/* Clone Role Dialog */}
      <Transition appear show={cloneDialogOpen} as={Fragment}>
        <Dialog as="div" className="relative z-50" onClose={() => setCloneDialogOpen(false)}>
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
                    Clone Role
                  </Dialog.Title>
                  <div className="mt-2">
                    <p className="text-sm text-gray-500">
                      Enter a name for the new role. All permissions from{' '}
                      <span className="font-medium">{roleToClone?.name}</span> will be copied.
                    </p>
                    <input
                      type="text"
                      value={cloneName}
                      onChange={(e) => setCloneName(e.target.value)}
                      className="mt-4 block w-full rounded-md border border-gray-300 px-3 py-2 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                      placeholder="New role name"
                    />
                  </div>

                  <div className="mt-4 flex justify-end gap-3">
                    <button
                      onClick={() => setCloneDialogOpen(false)}
                      className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={confirmClone}
                      disabled={cloneRole.isPending || !cloneName.trim()}
                      className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {cloneRole.isPending ? 'Cloning...' : 'Clone Role'}
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
