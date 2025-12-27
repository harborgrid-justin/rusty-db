import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'react-hot-toast';
import { userService } from '../services/userService';
import type {
  UUID,
  PaginationParams,
  UserStatus,
} from '@/types';
import type {
  UserFilters,
  CreateUserRequest,
  UpdateUserRequest,
  ResetPasswordRequest,
  BulkActionRequest,

} from '../services/userService';

// ============================================================================
// Query Keys
// ============================================================================

export const userKeys = {
  all: ['users'] as const,
  lists: () => [...userKeys.all, 'list'] as const,
  list: (filters: UserFilters, pagination: PaginationParams) =>
    [...userKeys.lists(), filters, pagination] as const,
  details: () => [...userKeys.all, 'detail'] as const,
  detail: (id: UUID) => [...userKeys.details(), id] as const,
  sessions: (id: UUID) => [...userKeys.detail(id), 'sessions'] as const,
  activity: (id: UUID) => [...userKeys.detail(id), 'activity'] as const,
  stats: () => [...userKeys.all, 'stats'] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch paginated list of users
 */
export function useUsers(
  filters: UserFilters = {},
  pagination: PaginationParams = { page: 1, pageSize: 20 }
) {
  return useQuery({
    queryKey: userKeys.list(filters, pagination),
    queryFn: () => userService.getUsers(filters, pagination),
    staleTime: 30000, // 30 seconds
  });
}

/**
 * Fetch single user by ID
 */
export function useUser(id: UUID | null) {
  return useQuery({
    queryKey: userKeys.detail(id!),
    queryFn: () => userService.getUser(id!),
    enabled: !!id,
    staleTime: 60000, // 1 minute
  });
}

/**
 * Fetch user sessions
 */
export function useUserSessions(userId: UUID | null) {
  return useQuery({
    queryKey: userKeys.sessions(userId!),
    queryFn: () => userService.getUserSessions(userId!),
    enabled: !!userId,
    refetchInterval: 10000, // Auto-refresh every 10 seconds
  });
}

/**
 * Fetch user activity log
 */
export function useUserActivity(
  userId: UUID | null,
  pagination: PaginationParams = { page: 1, pageSize: 50 }
) {
  return useQuery({
    queryKey: [...userKeys.activity(userId!), pagination],
    queryFn: () => userService.getUserActivity(userId!, pagination),
    enabled: !!userId,
    staleTime: 30000,
  });
}

/**
 * Fetch user statistics
 */
export function useUserStats() {
  return useQuery({
    queryKey: userKeys.stats(),
    queryFn: () => userService.getUserStats(),
    staleTime: 60000, // 1 minute
    refetchInterval: 60000, // Auto-refresh every minute
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create new user
 */
export function useCreateUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateUserRequest) => userService.createUser(data),
    onSuccess: (newUser) => {
      queryClient.invalidateQueries({ queryKey: userKeys.lists() });
      queryClient.invalidateQueries({ queryKey: userKeys.stats() });
      toast.success(`User "${newUser.username}" created successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to create user: ${error.message}`);
    },
  });
}

/**
 * Update existing user
 */
export function useUpdateUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: UUID; data: UpdateUserRequest }) =>
      userService.updateUser(id, data),
    onSuccess: (updatedUser) => {
      queryClient.invalidateQueries({ queryKey: userKeys.lists() });
      queryClient.invalidateQueries({ queryKey: userKeys.detail(updatedUser.id) });
      toast.success(`User "${updatedUser.username}" updated successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update user: ${error.message}`);
    },
  });
}

/**
 * Delete user
 */
export function useDeleteUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: UUID) => userService.deleteUser(id),
    onSuccess: (_, deletedId) => {
      queryClient.invalidateQueries({ queryKey: userKeys.lists() });
      queryClient.invalidateQueries({ queryKey: userKeys.stats() });
      queryClient.removeQueries({ queryKey: userKeys.detail(deletedId) });
      toast.success('User deleted successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete user: ${error.message}`);
    },
  });
}

/**
 * Reset user password
 */
export function useResetPassword() {
  return useMutation({
    mutationFn: (request: ResetPasswordRequest) =>
      userService.resetPassword(request),
    onSuccess: (response) => {
      if (response.temporaryPassword) {
        toast.success(
          `Password reset. Temporary password: ${response.temporaryPassword}`,
          { duration: 10000 }
        );
      } else {
        toast.success('Password reset email sent successfully');
      }
    },
    onError: (error: Error) => {
      toast.error(`Failed to reset password: ${error.message}`);
    },
  });
}

/**
 * Toggle user status
 */
export function useToggleUserStatus() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, status }: { id: UUID; status: UserStatus }) =>
      userService.toggleUserStatus(id, status),
    onSuccess: (updatedUser) => {
      queryClient.invalidateQueries({ queryKey: userKeys.lists() });
      queryClient.invalidateQueries({ queryKey: userKeys.detail(updatedUser.id) });
      queryClient.invalidateQueries({ queryKey: userKeys.stats() });
      toast.success(`User status updated to ${updatedUser.status}`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update status: ${error.message}`);
    },
  });
}

/**
 * Terminate user session
 */
export function useTerminateSession() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (sessionId: UUID) => userService.terminateSession(sessionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: userKeys.all });
      toast.success('Session terminated successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to terminate session: ${error.message}`);
    },
  });
}

/**
 * Terminate all sessions for a user
 */
export function useTerminateAllUserSessions() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (userId: UUID) => userService.terminateAllUserSessions(userId),
    onSuccess: (_, userId) => {
      queryClient.invalidateQueries({ queryKey: userKeys.sessions(userId) });
      toast.success('All sessions terminated successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to terminate sessions: ${error.message}`);
    },
  });
}

/**
 * Perform bulk action on multiple users
 */
export function useBulkUserAction() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: BulkActionRequest) => userService.bulkAction(request),
    onSuccess: (response) => {
      queryClient.invalidateQueries({ queryKey: userKeys.lists() });
      queryClient.invalidateQueries({ queryKey: userKeys.stats() });

      if (response.failed === 0) {
        toast.success(
          `Bulk action completed successfully. ${response.success} users affected.`
        );
      } else {
        toast.warning(
          `Bulk action completed with errors. Success: ${response.success}, Failed: ${response.failed}`
        );
      }
    },
    onError: (error: Error) => {
      toast.error(`Bulk action failed: ${error.message}`);
    },
  });
}

/**
 * Unlock user account
 */
export function useUnlockUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (userId: UUID) => userService.unlockUser(userId),
    onSuccess: (updatedUser) => {
      queryClient.invalidateQueries({ queryKey: userKeys.lists() });
      queryClient.invalidateQueries({ queryKey: userKeys.detail(updatedUser.id) });
      queryClient.invalidateQueries({ queryKey: userKeys.stats() });
      toast.success(`User "${updatedUser.username}" unlocked successfully`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to unlock user: ${error.message}`);
    },
  });
}

/**
 * Require password change on next login
 */
export function useRequirePasswordChange() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (userId: UUID) => userService.requirePasswordChange(userId),
    onSuccess: (_, userId) => {
      queryClient.invalidateQueries({ queryKey: userKeys.detail(userId) });
      toast.success('User will be required to change password on next login');
    },
    onError: (error: Error) => {
      toast.error(`Failed to require password change: ${error.message}`);
    },
  });
}

/**
 * Check username availability (debounced)
 */
export function useCheckUsername(username: string, enabled = true) {
  return useQuery({
    queryKey: ['checkUsername', username],
    queryFn: () => userService.checkUsernameAvailability(username),
    enabled: enabled && username.length >= 3,
    staleTime: 5000,
  });
}

/**
 * Check email availability (debounced)
 */
export function useCheckEmail(email: string, enabled = true) {
  return useQuery({
    queryKey: ['checkEmail', email],
    queryFn: () => userService.checkEmailAvailability(email),
    enabled: enabled && email.includes('@'),
    staleTime: 5000,
  });
}
