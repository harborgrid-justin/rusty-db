import { get, post, put, del } from './api';
import type { Role, Permission, UUID, PermissionAction } from '../types';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface CreateRoleRequest {
  name: string;
  description?: string;
  permissionIds: UUID[];
}

export interface UpdateRoleRequest {
  name?: string;
  description?: string;
  permissionIds?: UUID[];
}

export interface AssignRoleRequest {
  userId: UUID;
  roleId: UUID;
}

export interface RoleWithStats extends Role {
  userCount: number;
  permissionCount: number;
}

export interface PermissionGroup {
  resource: string;
  permissions: Permission[];
}

export interface RoleHierarchy {
  role: Role;
  inheritsFrom?: UUID[];
  inheritedBy?: UUID[];
  effectivePermissions: Permission[];
}

// ============================================================================
// Role Service
// ============================================================================

export const roleService = {
  /**
   * Get all roles with optional stats
   */
  async getRoles(includeStats = false): Promise<RoleWithStats[]> {
    const url = includeStats ? '/roles?includeStats=true' : '/roles';
    const response = await get<RoleWithStats[]>(url);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch roles');
    }

    return response.data;
  },

  /**
   * Get single role by ID with full details
   */
  async getRole(id: UUID): Promise<Role> {
    const response = await get<Role>(`/roles/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch role');
    }

    return response.data;
  },

  /**
   * Get role hierarchy information
   */
  async getRoleHierarchy(id: UUID): Promise<RoleHierarchy> {
    const response = await get<RoleHierarchy>(`/roles/${id}/hierarchy`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch role hierarchy');
    }

    return response.data;
  },

  /**
   * Create new role
   */
  async createRole(data: CreateRoleRequest): Promise<Role> {
    const response = await post<Role>('/roles', data);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create role');
    }

    return response.data;
  },

  /**
   * Update existing role
   */
  async updateRole(id: UUID, data: UpdateRoleRequest): Promise<Role> {
    const response = await put<Role>(`/roles/${id}`, data);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update role');
    }

    return response.data;
  },

  /**
   * Delete role (only if not a system role and not assigned to users)
   */
  async deleteRole(id: UUID): Promise<void> {
    const response = await del(`/roles/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete role');
    }
  },

  /**
   * Assign role to user
   */
  async assignRole(userId: UUID, roleId: UUID): Promise<void> {
    const response = await post('/roles/assign', { userId, roleId });

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to assign role');
    }
  },

  /**
   * Revoke role from user
   */
  async revokeRole(userId: UUID, roleId: UUID): Promise<void> {
    const response = await post('/roles/revoke', { userId, roleId });

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to revoke role');
    }
  },

  /**
   * Get users assigned to a role
   */
  async getRoleUsers(roleId: UUID): Promise<
    Array<{
      id: UUID;
      username: string;
      email?: string;
      displayName?: string;
      assignedAt: string;
    }>
  > {
    const response = await get<
      Array<{
        id: UUID;
        username: string;
        email?: string;
        displayName?: string;
        assignedAt: string;
      }>
    >(`/roles/${roleId}/users`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch role users');
    }

    return response.data;
  },

  /**
   * Check if role name is available
   */
  async checkRoleNameAvailability(name: string): Promise<boolean> {
    try {
      const response = await get<{ available: boolean }>(
        `/roles/check-name?name=${encodeURIComponent(name)}`
      );

      return response.success && response.data?.available === true;
    } catch {
      return false;
    }
  },

  /**
   * Clone role (create new role with same permissions)
   */
  async cloneRole(roleId: UUID, newName: string): Promise<Role> {
    const response = await post<Role>(`/roles/${roleId}/clone`, { name: newName });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to clone role');
    }

    return response.data;
  },
};

// ============================================================================
// Permission Service
// ============================================================================

export const permissionService = {
  /**
   * Get all available permissions
   */
  async getPermissions(): Promise<Permission[]> {
    const response = await get<Permission[]>('/permissions');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch permissions');
    }

    return response.data;
  },

  /**
   * Get permissions grouped by resource
   */
  async getPermissionsGrouped(): Promise<PermissionGroup[]> {
    const response = await get<PermissionGroup[]>('/permissions/grouped');

    if (!response.success || !response.data) {
      throw new Error(
        response.error?.message || 'Failed to fetch grouped permissions'
      );
    }

    return response.data;
  },

  /**
   * Get permission by ID
   */
  async getPermission(id: UUID): Promise<Permission> {
    const response = await get<Permission>(`/permissions/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch permission');
    }

    return response.data;
  },

  /**
   * Get all available resources
   */
  async getResources(): Promise<string[]> {
    const response = await get<string[]>('/permissions/resources');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch resources');
    }

    return response.data;
  },

  /**
   * Get all available actions
   */
  async getActions(): Promise<PermissionAction[]> {
    return ['create', 'read', 'update', 'delete', 'execute', 'grant', 'admin'];
  },

  /**
   * Create custom permission (admin only)
   */
  async createPermission(data: {
    name: string;
    resource: string;
    action: PermissionAction;
    description?: string;
  }): Promise<Permission> {
    const response = await post<Permission>('/permissions', data);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create permission');
    }

    return response.data;
  },

  /**
   * Update permission
   */
  async updatePermission(
    id: UUID,
    data: {
      name?: string;
      description?: string;
    }
  ): Promise<Permission> {
    const response = await put<Permission>(`/permissions/${id}`, data);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update permission');
    }

    return response.data;
  },

  /**
   * Delete permission
   */
  async deletePermission(id: UUID): Promise<void> {
    const response = await del(`/permissions/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete permission');
    }
  },

  /**
   * Get permission matrix (all resources x actions)
   */
  async getPermissionMatrix(): Promise<{
    resources: string[];
    actions: PermissionAction[];
    matrix: Record<string, Record<PermissionAction, Permission | null>>;
  }> {
    const response = await get<{
      resources: string[];
      actions: PermissionAction[];
      matrix: Record<string, Record<PermissionAction, Permission | null>>;
    }>('/permissions/matrix');

    if (!response.success || !response.data) {
      throw new Error(
        response.error?.message || 'Failed to fetch permission matrix'
      );
    }

    return response.data;
  },

  /**
   * Check if user has specific permission
   */
  async checkUserPermission(
    userId: UUID,
    resource: string,
    action: PermissionAction
  ): Promise<boolean> {
    try {
      const response = await get<{ hasPermission: boolean }>(
        `/permissions/check?userId=${userId}&resource=${encodeURIComponent(
          resource
        )}&action=${action}`
      );

      return response.success && response.data?.hasPermission === true;
    } catch {
      return false;
    }
  },
};
