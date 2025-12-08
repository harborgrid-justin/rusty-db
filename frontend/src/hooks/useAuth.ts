import { useAuthContext } from '../contexts/AuthContext';

/**
 * Hook for accessing authentication state and methods
 */
export function useAuth() {
  return useAuthContext();
}
