import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  type ReactNode,
} from 'react';
import type { User, Session, LoginCredentials, AuthState } from '../types';
import { authService } from '../services/authService';

interface AuthContextType extends AuthState {
  login: (credentials: LoginCredentials) => Promise<void>;
  logout: () => Promise<void>;
  refreshSession: () => Promise<void>;
  updateUser: (user: Partial<User>) => void;
  hasPermission: (resource: string, action: string) => boolean;
  hasRole: (roleName: string) => boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

const AUTH_STORAGE_KEY = 'rustydb_auth';
const SESSION_CHECK_INTERVAL = 60000; // 1 minute

interface StoredAuth {
  user: User;
  session: Session;
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AuthState>({
    user: null,
    session: null,
    isAuthenticated: false,
    isLoading: true,
    error: null,
  });

  // Initialize auth state from storage
  useEffect(() => {
    const initializeAuth = async () => {
      try {
        const storedAuth = localStorage.getItem(AUTH_STORAGE_KEY);

        if (storedAuth) {
          const { user, session } = JSON.parse(storedAuth) as StoredAuth;

          // Check if session is still valid
          if (new Date(session.expiresAt) > new Date()) {
            // Verify session with server
            const isValid = await authService.validateSession(session.token);

            if (isValid) {
              setState({
                user,
                session,
                isAuthenticated: true,
                isLoading: false,
                error: null,
              });
              return;
            }
          }

          // Session expired or invalid, clear storage
          localStorage.removeItem(AUTH_STORAGE_KEY);
        }

        // Check if auth is disabled (development mode)
        const authEnabled = import.meta.env.VITE_AUTH_ENABLED !== 'false';

        if (!authEnabled) {
          // Create mock user for development
          const mockUser: User = {
            id: 'dev-user',
            username: 'developer',
            displayName: 'Developer',
            email: 'dev@localhost',
            roles: [
              {
                id: 'admin',
                name: 'Admin',
                permissions: [],
                isSystem: true,
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            ],
            permissions: [],
            status: 'active',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            settings: {
              theme: 'dark',
              timezone: 'UTC',
              language: 'en',
              notifications: {
                email: false,
                browser: true,
                alertsOnly: false,
              },
            },
          };

          setState({
            user: mockUser,
            session: null,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });
          return;
        }

        setState((prev) => ({ ...prev, isLoading: false }));
      } catch (error) {
        console.error('Failed to initialize auth:', error);
        localStorage.removeItem(AUTH_STORAGE_KEY);
        setState({
          user: null,
          session: null,
          isAuthenticated: false,
          isLoading: false,
          error: 'Failed to initialize authentication',
        });
      }
    };

    initializeAuth();
  }, []);

  // Periodic session validation
  useEffect(() => {
    if (!state.isAuthenticated || !state.session) return;

    const checkSession = async () => {
      try {
        const isValid = await authService.validateSession(state.session!.token);

        if (!isValid) {
          // Try to refresh session
          try {
            await refreshSession();
          } catch {
            // Refresh failed, log out
            await logout();
          }
        }
      } catch (error) {
        console.error('Session check failed:', error);
      }
    };

    const interval = setInterval(checkSession, SESSION_CHECK_INTERVAL);
    return () => clearInterval(interval);
  }, [state.isAuthenticated, state.session]);

  const login = useCallback(async (credentials: LoginCredentials) => {
    setState((prev) => ({ ...prev, isLoading: true, error: null }));

    try {
      const { user, session } = await authService.login(credentials);

      // Store auth data
      localStorage.setItem(
        AUTH_STORAGE_KEY,
        JSON.stringify({ user, session })
      );

      setState({
        user,
        session,
        isAuthenticated: true,
        isLoading: false,
        error: null,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Login failed';
      setState((prev) => ({
        ...prev,
        isLoading: false,
        error: message,
      }));
      throw error;
    }
  }, []);

  const logout = useCallback(async () => {
    try {
      if (state.session) {
        await authService.logout(state.session.token);
      }
    } catch (error) {
      console.error('Logout error:', error);
    } finally {
      localStorage.removeItem(AUTH_STORAGE_KEY);
      setState({
        user: null,
        session: null,
        isAuthenticated: false,
        isLoading: false,
        error: null,
      });
    }
  }, [state.session]);

  const refreshSession = useCallback(async () => {
    if (!state.session?.refreshToken) {
      throw new Error('No refresh token available');
    }

    try {
      const newSession = await authService.refreshSession(
        state.session.refreshToken
      );

      const updatedAuth = {
        user: state.user!,
        session: newSession,
      };

      localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(updatedAuth));

      setState((prev) => ({
        ...prev,
        session: newSession,
      }));
    } catch (error) {
      // Refresh failed, clear auth
      await logout();
      throw error;
    }
  }, [state.session, state.user, logout]);

  const updateUser = useCallback((updates: Partial<User>) => {
    setState((prev) => {
      if (!prev.user) return prev;

      const updatedUser = { ...prev.user, ...updates };

      if (prev.session) {
        localStorage.setItem(
          AUTH_STORAGE_KEY,
          JSON.stringify({ user: updatedUser, session: prev.session })
        );
      }

      return { ...prev, user: updatedUser };
    });
  }, []);

  const hasPermission = useCallback(
    (resource: string, action: string): boolean => {
      if (!state.user) return false;

      // Check if user has admin role (full access)
      if (state.user.roles.some((role) => role.name === 'Admin')) {
        return true;
      }

      // Check direct permissions
      const hasDirectPermission = state.user.permissions.some(
        (perm) => perm.resource === resource && perm.action === action
      );

      if (hasDirectPermission) return true;

      // Check role permissions
      return state.user.roles.some((role) =>
        role.permissions.some(
          (perm) => perm.resource === resource && perm.action === action
        )
      );
    },
    [state.user]
  );

  const hasRole = useCallback(
    (roleName: string): boolean => {
      if (!state.user) return false;
      return state.user.roles.some((role) => role.name === roleName);
    },
    [state.user]
  );

  const contextValue: AuthContextType = {
    ...state,
    login,
    logout,
    refreshSession,
    updateUser,
    hasPermission,
    hasRole,
  };

  return (
    <AuthContext.Provider value={contextValue}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuthContext(): AuthContextType {
  const context = useContext(AuthContext);

  if (context === undefined) {
    throw new Error('useAuthContext must be used within an AuthProvider');
  }

  return context;
}
