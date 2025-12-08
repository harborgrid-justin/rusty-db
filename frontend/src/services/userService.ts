import { get, post, put, del, buildQueryParams } from './api';
import type {
  User,
  UUID,
  PaginatedResponse,
  PaginationParams,
  UserStatus,
  ActiveSession,
} from '../types';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface UserFilters {
  search?: string;
  status?: UserStatus;
  roleId?: UUID;
  createdAfter?: string;
  createdBefore?: string;
}

export interface CreateUserRequest {
  username: string;
  email?: string;
  displayName?: string;
  password: string;
  roleIds: UUID[];
  status?: UserStatus;
}

export interface UpdateUserRequest {
  email?: string;
  displayName?: string;
  roleIds?: UUID[];
  status?: UserStatus;
}

export interface ResetPasswordRequest {
  userId: UUID;
  newPassword?: string; // If not provided, generates temporary password
  requireChangeOnLogin?: boolean;
}

export interface ResetPasswordResponse {
  temporaryPassword?: string;
  requireChangeOnLogin: boolean;
  expiresAt?: string;
}

export interface BulkActionRequest {
  userIds: UUID[];
  action: 'activate' | 'deactivate' | 'lock' | 'unlock' | 'delete';
}

export interface BulkActionResponse {
  success: number;
  failed: number;
  errors?: Array<{ userId: UUID; error: string }>;
}

export interface UserSessionInfo extends ActiveSession {
  username: string;
  email?: string;
}

// ============================================================================
// User Service
// ============================================================================

export const userService = {
  /**
   * Get paginated list of users with filters
   */
  async getUsers(
    filters: UserFilters = {},
    pagination: PaginationParams = { page: 1, pageSize: 20 }
  ): Promise<PaginatedResponse<User>> {
    const queryParams = buildQueryParams({
      ...filters,
      ...pagination,
    });

    const response = await get<PaginatedResponse<User>>(`/users${queryParams}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch users');
    }

    return response.data;
  },

  /**
   * Get single user by ID with full details
   */
  async getUser(id: UUID): Promise<User> {
    const response = await get<User>(`/users/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch user');
    }

    return response.data;
  },

  /**
   * Create new user
   */
  async createUser(data: CreateUserRequest): Promise<User> {
    const response = await post<User>('/users', data);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create user');
    }

    return response.data;
  },

  /**
   * Update existing user
   */
  async updateUser(id: UUID, data: UpdateUserRequest): Promise<User> {
    const response = await put<User>(`/users/${id}`, data);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update user');
    }

    return response.data;
  },

  /**
   * Delete user
   */
  async deleteUser(id: UUID): Promise<void> {
    const response = await del(`/users/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete user');
    }
  },

  /**
   * Reset user password (admin action)
   */
  async resetPassword(
    request: ResetPasswordRequest
  ): Promise<ResetPasswordResponse> {
    const response = await post<ResetPasswordResponse>(
      `/users/${request.userId}/reset-password`,
      {
        newPassword: request.newPassword,
        requireChangeOnLogin: request.requireChangeOnLogin ?? true,
      }
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to reset password');
    }

    return response.data;
  },

  /**
   * Toggle user status (active/inactive/locked)
   */
  async toggleUserStatus(id: UUID, status: UserStatus): Promise<User> {
    const response = await put<User>(`/users/${id}/status`, { status });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update user status');
    }

    return response.data;
  },

  /**
   * Get all active sessions for a user
   */
  async getUserSessions(id: UUID): Promise<UserSessionInfo[]> {
    const response = await get<UserSessionInfo[]>(`/users/${id}/sessions`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch user sessions');
    }

    return response.data;
  },

  /**
   * Terminate a specific user session
   */
  async terminateSession(sessionId: UUID): Promise<void> {
    const response = await del(`/sessions/${sessionId}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to terminate session');
    }
  },

  /**
   * Terminate all sessions for a user
   */
  async terminateAllUserSessions(userId: UUID): Promise<void> {
    const response = await del(`/users/${userId}/sessions`);

    if (!response.success) {
      throw new Error(
        response.error?.message || 'Failed to terminate user sessions'
      );
    }
  },

  /**
   * Perform bulk actions on multiple users
   */
  async bulkAction(request: BulkActionRequest): Promise<BulkActionResponse> {
    const response = await post<BulkActionResponse>('/users/bulk', request);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to perform bulk action');
    }

    return response.data;
  },

  /**
   * Check if username is available
   */
  async checkUsernameAvailability(username: string): Promise<boolean> {
    try {
      const response = await get<{ available: boolean }>(
        `/users/check-username?username=${encodeURIComponent(username)}`
      );

      return response.success && response.data?.available === true;
    } catch {
      return false;
    }
  },

  /**
   * Check if email is available
   */
  async checkEmailAvailability(email: string): Promise<boolean> {
    try {
      const response = await get<{ available: boolean }>(
        `/users/check-email?email=${encodeURIComponent(email)}`
      );

      return response.success && response.data?.available === true;
    } catch {
      return false;
    }
  },

  /**
   * Get user activity log
   */
  async getUserActivity(
    userId: UUID,
    pagination: PaginationParams = { page: 1, pageSize: 50 }
  ): Promise<
    PaginatedResponse<{
      id: UUID;
      timestamp: string;
      action: string;
      resource: string;
      details?: Record<string, unknown>;
    }>
  > {
    const queryParams = buildQueryParams(pagination);
    const response = await get<
      PaginatedResponse<{
        id: UUID;
        timestamp: string;
        action: string;
        resource: string;
        details?: Record<string, unknown>;
      }>
    >(`/users/${userId}/activity${queryParams}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch user activity');
    }

    return response.data;
  },

  /**
   * Force user to change password on next login
   */
  async requirePasswordChange(userId: UUID): Promise<void> {
    const response = await post(`/users/${userId}/require-password-change`, {});

    if (!response.success) {
      throw new Error(
        response.error?.message || 'Failed to require password change'
      );
    }
  },

  /**
   * Unlock user account (if locked due to failed login attempts)
   */
  async unlockUser(userId: UUID): Promise<User> {
    const response = await post<User>(`/users/${userId}/unlock`, {});

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to unlock user');
    }

    return response.data;
  },

  /**
   * Get user statistics
   */
  async getUserStats(): Promise<{
    total: number;
    active: number;
    inactive: number;
    locked: number;
    pending: number;
    lastDay: number;
    lastWeek: number;
    lastMonth: number;
  }> {
    const response = await get<{
      total: number;
      active: number;
      inactive: number;
      locked: number;
      pending: number;
      lastDay: number;
      lastWeek: number;
      lastMonth: number;
    }>('/users/stats');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch user stats');
    }

    return response.data;
  },
};
