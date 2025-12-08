import { post, get } from './api';
import type { User, Session, LoginCredentials } from '../types';

export interface LoginResponse {
  user: User;
  session: Session;
}

export const authService = {
  /**
   * Authenticate user with credentials
   */
  async login(credentials: LoginCredentials): Promise<LoginResponse> {
    const response = await post<LoginResponse>('/auth/login', credentials);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Login failed');
    }

    return response.data;
  },

  /**
   * Log out current session
   */
  async logout(token: string): Promise<void> {
    await post('/auth/logout', { token });
  },

  /**
   * Refresh session with refresh token
   */
  async refreshSession(refreshToken: string): Promise<Session> {
    const response = await post<Session>('/auth/refresh', { refreshToken });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Session refresh failed');
    }

    return response.data;
  },

  /**
   * Validate session token
   */
  async validateSession(token: string): Promise<boolean> {
    try {
      const response = await get<{ valid: boolean }>('/auth/validate', {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      return response.success && response.data?.valid === true;
    } catch {
      return false;
    }
  },

  /**
   * Get current user profile
   */
  async getCurrentUser(): Promise<User> {
    const response = await get<User>('/auth/me');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to get user profile');
    }

    return response.data;
  },

  /**
   * Update user password
   */
  async changePassword(
    currentPassword: string,
    newPassword: string
  ): Promise<void> {
    const response = await post('/auth/change-password', {
      currentPassword,
      newPassword,
    });

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to change password');
    }
  },

  /**
   * Request password reset
   */
  async requestPasswordReset(email: string): Promise<void> {
    const response = await post('/auth/forgot-password', { email });

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to request password reset');
    }
  },

  /**
   * Reset password with token
   */
  async resetPassword(token: string, newPassword: string): Promise<void> {
    const response = await post('/auth/reset-password', {
      token,
      newPassword,
    });

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to reset password');
    }
  },

  /**
   * Update user settings
   */
  async updateSettings(settings: Partial<User['settings']>): Promise<User> {
    const response = await post<User>('/auth/settings', settings);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update settings');
    }

    return response.data;
  },
};
