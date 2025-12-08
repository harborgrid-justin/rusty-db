import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'react-hot-toast';
import { roleService, permissionService } from '../services/roleService';
import type { Role, Permission, UUID, PermissionAction } from '../types';
import type {
  CreateRoleRequest,
  UpdateRoleRequest,
  RoleWithStats,
} from '../services/roleService';

// ============================================================================
// Query Keys
// ============================================================================

export const roleKeys = {
  all: ['roles'] as const,
  lists: () => [...roleKeys.all, 'list'] as const,
  list: (includeStats: boolean) => [...roleKeys.lists(), { includeStats }] as const,
  details: () => [...roleKeys.all, 'detail'] as const,
  detail: (id: UUID) => [...roleKeys.details(), id] as const,
  hierarchy: (id: UUID) => [...roleKeys.detail(id), 'hierarchy'] as const,
  users: (id: UUID) => [...roleKeys.detail(id), 'users'] as const,
};

export const permissionKeys = {
  all: ['permissions'] as const,
  lists: () => [...permissionKeys.all, 'list'] as const,
  list: () => [...permissionKeys.lists()] as const,
  grouped: () => [...permissionKeys.lists(), 'grouped'] as const,
  matrix: () => [...permissionKeys.all, 'matrix'] as const,
  resources: () => [...permissionKeys.all, 'resources'] as const,
  actions: () => [...permissionKeys.all, 'actions'] as const,
};

// ============================================================================
// Role Query Hooks
// ============================================================================

/**
 * Fetch all roles
 */
export function useRoles(includeStats = false) {
  return useQuery({
    queryKey: roleKeys.list(includeStats),
    queryFn: () => roleService.getRoles(includeStats),
    staleTime: 60000, // 1 minute
  });
}

/**
 * Fetch single role by ID
 */
export function useRole(id: UUID | null) {
  return useQuery({
    queryKey: roleKeys.detail(id!),
    queryFn: () => roleService.getRole(id!),
    enabled: !!id,
    staleTime: 60000,
  });
}

/**
 * Fetch role hierarchy
 */
export function useRoleHierarchy(id: UUID | null) {
  return useQuery({
    queryKey: roleKeys.hierarchy(id!),
    queryFn: () => roleService.getRoleHierarchy(id!),
    enabled: !!id,
    staleTime: 60000,
  });
}

/**
 * Fetch users assigned to a role
 */
export function useRoleUsers(roleId: UUID | null) {
  return useQuery({
    queryKey: roleKeys.users(roleId!),
    queryFn: () => roleService.getRoleUsers(roleId!),
    enabled: !!roleId,
    staleTime: 30000,
  });
}

/**
 * Check role name availability
 */
export function useCheckRoleName(name: string, enabled = true) {
  return useQuery({
    queryKey: ['checkRoleName', name],
    queryFn: () => roleService.checkRoleNameAvailability(name),
    enabled: enabled && name.length >= 2,
    staleTime: 5000,
  });
}

// ============================================================================
// Role Mutation Hooks
// ============================================================================

/**
 * Create new role
 */
export function useCreateRole() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateRoleRequest) => roleService.createRole(data),
    onSuccess: (newRole) => {
      queryClient.invalidateQueries({ queryKey: roleKeys.lists() });
      toast.success(`Role "${newRole.name}" created successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to create role: ${error.message}`);
    },
  });
}

/**
 * Update existing role
 */
export function useUpdateRole() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: UUID; data: UpdateRoleRequest }) =>
      roleService.updateRole(id, data),
    onSuccess: (updatedRole) => {
      queryClient.invalidateQueries({ queryKey: roleKeys.lists() });
      queryClient.invalidateQueries({ queryKey: roleKeys.detail(updatedRole.id) });
      queryClient.invalidateQueries({
        queryKey: roleKeys.hierarchy(updatedRole.id),
      });
      toast.success(`Role "${updatedRole.name}" updated successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update role: ${error.message}`);
    },
  });
}

/**
 * Delete role
 */
export function useDeleteRole() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: UUID) => roleService.deleteRole(id),
    onSuccess: (_, deletedId) => {
      queryClient.invalidateQueries({ queryKey: roleKeys.lists() });
      queryClient.removeQueries({ queryKey: roleKeys.detail(deletedId) });
      toast.success('Role deleted successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete role: ${error.message}`);
    },
  });
}

/**
 * Assign role to user
 */
export function useAssignRole() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ userId, roleId }: { userId: UUID; roleId: UUID }) =>
      roleService.assignRole(userId, roleId),
    onSuccess: (_, { roleId }) => {
      queryClient.invalidateQueries({ queryKey: roleKeys.users(roleId) });
      queryClient.invalidateQueries({ queryKey: roleKeys.lists() });
      queryClient.invalidateQueries({ queryKey: ['users'] });
      toast.success('Role assigned successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to assign role: ${error.message}`);
    },
  });
}

/**
 * Revoke role from user
 */
export function useRevokeRole() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ userId, roleId }: { userId: UUID; roleId: UUID }) =>
      roleService.revokeRole(userId, roleId),
    onSuccess: (_, { roleId }) => {
      queryClient.invalidateQueries({ queryKey: roleKeys.users(roleId) });
      queryClient.invalidateQueries({ queryKey: roleKeys.lists() });
      queryClient.invalidateQueries({ queryKey: ['users'] });
      toast.success('Role revoked successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to revoke role: ${error.message}`);
    },
  });
}

/**
 * Clone role
 */
export function useCloneRole() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ roleId, newName }: { roleId: UUID; newName: string }) =>
      roleService.cloneRole(roleId, newName),
    onSuccess: (newRole) => {
      queryClient.invalidateQueries({ queryKey: roleKeys.lists() });
      toast.success(`Role cloned as "${newRole.name}"`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to clone role: ${error.message}`);
    },
  });
}

// ============================================================================
// Permission Query Hooks
// ============================================================================

/**
 * Fetch all permissions
 */
export function usePermissions() {
  return useQuery({
    queryKey: permissionKeys.list(),
    queryFn: () => permissionService.getPermissions(),
    staleTime: 300000, // 5 minutes - permissions rarely change
  });
}

/**
 * Fetch permissions grouped by resource
 */
export function usePermissionsGrouped() {
  return useQuery({
    queryKey: permissionKeys.grouped(),
    queryFn: () => permissionService.getPermissionsGrouped(),
    staleTime: 300000,
  });
}

/**
 * Fetch permission matrix
 */
export function usePermissionMatrix() {
  return useQuery({
    queryKey: permissionKeys.matrix(),
    queryFn: () => permissionService.getPermissionMatrix(),
    staleTime: 300000,
  });
}

/**
 * Fetch available resources
 */
export function useResources() {
  return useQuery({
    queryKey: permissionKeys.resources(),
    queryFn: () => permissionService.getResources(),
    staleTime: 300000,
  });
}

/**
 * Fetch available actions
 */
export function useActions() {
  return useQuery({
    queryKey: permissionKeys.actions(),
    queryFn: () => permissionService.getActions(),
    staleTime: Infinity, // Actions are static
    initialData: ['create', 'read', 'update', 'delete', 'execute', 'grant', 'admin'],
  });
}

/**
 * Check if user has permission
 */
export function useCheckPermission(
  userId: UUID | null,
  resource: string,
  action: PermissionAction
) {
  return useQuery({
    queryKey: ['checkPermission', userId, resource, action],
    queryFn: () => permissionService.checkUserPermission(userId!, resource, action),
    enabled: !!userId && !!resource && !!action,
    staleTime: 60000,
  });
}

// ============================================================================
// Permission Mutation Hooks
// ============================================================================

/**
 * Create custom permission
 */
export function useCreatePermission() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: {
      name: string;
      resource: string;
      action: PermissionAction;
      description?: string;
    }) => permissionService.createPermission(data),
    onSuccess: (newPermission) => {
      queryClient.invalidateQueries({ queryKey: permissionKeys.all });
      toast.success(`Permission "${newPermission.name}" created successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to create permission: ${error.message}`);
    },
  });
}

/**
 * Update permission
 */
export function useUpdatePermission() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: UUID;
      data: { name?: string; description?: string };
    }) => permissionService.updatePermission(id, data),
    onSuccess: (updatedPermission) => {
      queryClient.invalidateQueries({ queryKey: permissionKeys.all });
      toast.success(`Permission "${updatedPermission.name}" updated successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update permission: ${error.message}`);
    },
  });
}

/**
 * Delete permission
 */
export function useDeletePermission() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: UUID) => permissionService.deletePermission(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: permissionKeys.all });
      toast.success('Permission deleted successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete permission: ${error.message}`);
    },
  });
}
