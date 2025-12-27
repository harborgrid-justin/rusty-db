import { Fragment, useEffect } from 'react';
import { Dialog, Transition } from '@headlessui/react';
import { useForm, Controller } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import {
  XMarkIcon,
  InformationCircleIcon,
  ShieldCheckIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { User, Role, UserStatus, UUID } from '../../types';
import type { CreateUserRequest, UpdateUserRequest } from '../../services/userService';

// ============================================================================
// Validation Schemas
// ============================================================================

const createUserSchema = z.object({
  username: z
    .string()
    .min(3, 'Username must be at least 3 characters')
    .max(50, 'Username must be less than 50 characters')
    .regex(/^[a-zA-Z0-9_-]+$/, 'Username can only contain letters, numbers, hyphens, and underscores'),
  email: z
    .string()
    .email('Invalid email address')
    .optional()
    .or(z.literal('')),
  displayName: z
    .string()
    .max(100, 'Display name must be less than 100 characters')
    .optional()
    .or(z.literal('')),
  password: z
    .string()
    .min(8, 'Password must be at least 8 characters')
    .regex(/[A-Z]/, 'Password must contain at least one uppercase letter')
    .regex(/[a-z]/, 'Password must contain at least one lowercase letter')
    .regex(/[0-9]/, 'Password must contain at least one number')
    .regex(/[^A-Za-z0-9]/, 'Password must contain at least one special character'),
  confirmPassword: z.string(),
  roleIds: z.array(z.string()).min(1, 'At least one role must be selected'),
  status: z.enum(['active', 'inactive', 'locked', 'pending']),
}).refine((data) => data.password === data.confirmPassword, {
  message: "Passwords don't match",
  path: ['confirmPassword'],
});

const updateUserSchema = z.object({
  email: z
    .string()
    .email('Invalid email address')
    .optional()
    .or(z.literal('')),
  displayName: z
    .string()
    .max(100, 'Display name must be less than 100 characters')
    .optional()
    .or(z.literal('')),
  roleIds: z.array(z.string()).min(1, 'At least one role must be selected'),
  status: z.enum(['active', 'inactive', 'locked', 'pending']),
});

type CreateFormData = z.infer<typeof createUserSchema>;
type UpdateFormData = z.infer<typeof updateUserSchema>;

// ============================================================================
// Props
// ============================================================================

interface UserFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: CreateUserRequest | UpdateUserRequest) => void;
  user?: User | null;
  roles: Role[];
  isLoading?: boolean;
}

// ============================================================================
// Helper Components
// ============================================================================

function PasswordRequirements() {
  return (
    <div className="rounded-md bg-blue-50 p-3">
      <div className="flex gap-2">
        <InformationCircleIcon className="h-5 w-5 flex-shrink-0 text-blue-400" />
        <div className="text-xs text-blue-700">
          <p className="font-medium">Password Requirements:</p>
          <ul className="mt-1 list-inside list-disc space-y-0.5">
            <li>At least 8 characters long</li>
            <li>Contains uppercase and lowercase letters</li>
            <li>Contains at least one number</li>
            <li>Contains at least one special character</li>
          </ul>
        </div>
      </div>
    </div>
  );
}

function RoleSelector({
  value,
  onChange,
  roles,
  error,
}: {
  value: UUID[];
  onChange: (value: UUID[]) => void;
  roles: Role[];
  error?: string;
}) {
  const toggleRole = (roleId: UUID) => {
    if (value.includes(roleId)) {
      onChange(value.filter((id) => id !== roleId));
    } else {
      onChange([...value, roleId]);
    }
  };

  return (
    <div>
      <label className="block text-sm font-medium text-gray-700">
        Roles <span className="text-red-500">*</span>
      </label>
      <div className="mt-1 space-y-2">
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {roles.map((role) => (
            <div
              key={role.id}
              className={clsx(
                'relative flex cursor-pointer rounded-lg border p-3 transition-colors',
                value.includes(role.id)
                  ? 'border-blue-600 bg-blue-50'
                  : 'border-gray-300 hover:border-gray-400',
                role.isSystem && 'opacity-75'
              )}
              onClick={() => !role.isSystem && toggleRole(role.id)}
            >
              <div className="flex flex-1 items-start gap-3">
                <input
                  type="checkbox"
                  checked={value.includes(role.id)}
                  onChange={() => !role.isSystem && toggleRole(role.id)}
                  disabled={role.isSystem}
                  className="mt-0.5 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600 disabled:cursor-not-allowed disabled:opacity-50"
                />
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-gray-900">
                      {role.name}
                    </span>
                    {role.isSystem && (
                      <span className="inline-flex items-center gap-1 rounded-md bg-gray-100 px-1.5 py-0.5 text-xs text-gray-600">
                        <ShieldCheckIcon className="h-3 w-3" />
                        System
                      </span>
                    )}
                  </div>
                  {role.description && (
                    <p className="mt-0.5 text-xs text-gray-500">{role.description}</p>
                  )}
                  <p className="mt-1 text-xs text-gray-400">
                    {role.permissions.length} permission
                    {role.permissions.length !== 1 ? 's' : ''}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
        {error && <p className="mt-1 text-sm text-red-600">{error}</p>}
      </div>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function UserForm({
  isOpen,
  onClose,
  onSubmit,
  user,
  roles,
  isLoading = false,
}: UserFormProps) {
  const isEditMode = !!user;

  const {
    register,
    handleSubmit,
    control,
    reset,
    formState: { errors },
  } = useForm<CreateFormData | UpdateFormData>({
    resolver: zodResolver(isEditMode ? updateUserSchema : createUserSchema),
    defaultValues: isEditMode
      ? {
          email: user.email || '',
          displayName: user.displayName || '',
          roleIds: user.roles.map((r) => r.id),
          status: user.status,
        }
      : {
          username: '',
          email: '',
          displayName: '',
          password: '',
          confirmPassword: '',
          roleIds: [],
          status: 'active' as UserStatus,
        },
  });

  // Reset form when user changes
  useEffect(() => {
    if (isOpen) {
      if (isEditMode && user) {
        reset({
          email: user.email || '',
          displayName: user.displayName || '',
          roleIds: user.roles.map((r) => r.id),
          status: user.status,
        });
      } else {
        reset({
          username: '',
          email: '',
          displayName: '',
          password: '',
          confirmPassword: '',
          roleIds: [],
          status: 'active',
        });
      }
    }
  }, [isOpen, isEditMode, user, reset]);

  const handleFormSubmit = (data: CreateFormData | UpdateFormData) => {
    if (isEditMode) {
      const updateData = data as UpdateFormData;
      onSubmit({
        email: updateData.email || undefined,
        displayName: updateData.displayName || undefined,
        roleIds: updateData.roleIds,
        status: updateData.status,
      });
    } else {
      const createData = data as CreateFormData;
      onSubmit({
        username: createData.username,
        email: createData.email || undefined,
        displayName: createData.displayName || undefined,
        password: createData.password,
        roleIds: createData.roleIds,
        status: createData.status,
      });
    }
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
          <div className="flex min-h-full items-center justify-center p-4 text-center">
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
                <Dialog.Title
                  as="div"
                  className="flex items-center justify-between border-b border-gray-200 pb-4"
                >
                  <h3 className="text-lg font-medium leading-6 text-gray-900">
                    {isEditMode ? `Edit User: ${user.username}` : 'Create New User'}
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
                  {/* Username - only for create */}
                  {!isEditMode && (
                    <div>
                      <label
                        htmlFor="username"
                        className="block text-sm font-medium text-gray-700"
                      >
                        Username <span className="text-red-500">*</span>
                      </label>
                      <input
                        {...register('username')}
                        type="text"
                        className={clsx(
                          'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                          errors.username
                            ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                            : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                        )}
                        placeholder="john_doe"
                      />
                      {errors.username && (
                        <p className="mt-1 text-sm text-red-600">
                          {errors.username.message}
                        </p>
                      )}
                    </div>
                  )}

                  {/* Email */}
                  <div>
                    <label
                      htmlFor="email"
                      className="block text-sm font-medium text-gray-700"
                    >
                      Email
                    </label>
                    <input
                      {...register('email')}
                      type="email"
                      className={clsx(
                        'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                        errors.email
                          ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                          : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                      )}
                      placeholder="john@example.com"
                    />
                    {errors.email && (
                      <p className="mt-1 text-sm text-red-600">{errors.email.message}</p>
                    )}
                  </div>

                  {/* Display Name */}
                  <div>
                    <label
                      htmlFor="displayName"
                      className="block text-sm font-medium text-gray-700"
                    >
                      Display Name
                    </label>
                    <input
                      {...register('displayName')}
                      type="text"
                      className={clsx(
                        'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                        errors.displayName
                          ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                          : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                      )}
                      placeholder="John Doe"
                    />
                    {errors.displayName && (
                      <p className="mt-1 text-sm text-red-600">
                        {errors.displayName.message}
                      </p>
                    )}
                  </div>

                  {/* Password - only for create */}
                  {!isEditMode && (
                    <>
                      <div>
                        <label
                          htmlFor="password"
                          className="block text-sm font-medium text-gray-700"
                        >
                          Password <span className="text-red-500">*</span>
                        </label>
                        <input
                          {...register('password')}
                          type="password"
                          className={clsx(
                            'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                            errors.password
                              ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                              : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                          )}
                          autoComplete="new-password"
                        />
                        {errors.password && (
                          <p className="mt-1 text-sm text-red-600">
                            {errors.password.message}
                          </p>
                        )}
                      </div>

                      <div>
                        <label
                          htmlFor="confirmPassword"
                          className="block text-sm font-medium text-gray-700"
                        >
                          Confirm Password <span className="text-red-500">*</span>
                        </label>
                        <input
                          {...register('confirmPassword')}
                          type="password"
                          className={clsx(
                            'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                            errors.confirmPassword
                              ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                              : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                          )}
                          autoComplete="new-password"
                        />
                        {errors.confirmPassword && (
                          <p className="mt-1 text-sm text-red-600">
                            {errors.confirmPassword.message}
                          </p>
                        )}
                      </div>

                      <PasswordRequirements />
                    </>
                  )}

                  {/* Role Selection */}
                  <Controller
                    name="roleIds"
                    control={control}
                    render={({ field }) => (
                      <RoleSelector
                        value={field.value}
                        onChange={field.onChange}
                        roles={roles}
                        error={errors.roleIds?.message}
                      />
                    )}
                  />

                  {/* Status */}
                  <div>
                    <label
                      htmlFor="status"
                      className="block text-sm font-medium text-gray-700"
                    >
                      Status <span className="text-red-500">*</span>
                    </label>
                    <select
                      {...register('status')}
                      className={clsx(
                        'mt-1 block w-full rounded-md border px-3 py-2 shadow-sm focus:outline-none focus:ring-2',
                        errors.status
                          ? 'border-red-300 focus:border-red-500 focus:ring-red-500'
                          : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                      )}
                    >
                      <option value="active">Active</option>
                      <option value="inactive">Inactive</option>
                      <option value="locked">Locked</option>
                      <option value="pending">Pending</option>
                    </select>
                    {errors.status && (
                      <p className="mt-1 text-sm text-red-600">{errors.status.message}</p>
                    )}
                  </div>

                  {/* Actions */}
                  <div className="flex justify-end gap-3 border-t border-gray-200 pt-4">
                    <button
                      type="button"
                      onClick={onClose}
                      className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      disabled={isLoading}
                      className="rounded-md border border-transparent bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {isLoading
                        ? 'Saving...'
                        : isEditMode
                        ? 'Update User'
                        : 'Create User'}
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
