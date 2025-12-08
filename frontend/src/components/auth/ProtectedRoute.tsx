import { Navigate, useLocation } from 'react-router-dom';
import type { ReactNode } from 'react';
import { useAuth } from '../../hooks/useAuth';
import { LoadingScreen } from '../common/LoadingScreen';

interface ProtectedRouteProps {
  children: ReactNode;
  requiredPermission?: {
    resource: string;
    action: string;
  };
  requiredRole?: string;
}

export function ProtectedRoute({
  children,
  requiredPermission,
  requiredRole,
}: ProtectedRouteProps) {
  const { isAuthenticated, isLoading, hasPermission, hasRole } = useAuth();
  const location = useLocation();

  // Show loading while checking auth
  if (isLoading) {
    return <LoadingScreen message="Checking authentication..." />;
  }

  // Redirect to login if not authenticated
  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  // Check required permission
  if (requiredPermission) {
    if (!hasPermission(requiredPermission.resource, requiredPermission.action)) {
      return (
        <div className="min-h-screen bg-dark-900 flex items-center justify-center">
          <div className="max-w-md w-full bg-dark-800 border border-dark-700 rounded-xl p-8 text-center">
            <h1 className="text-xl font-semibold text-dark-100 mb-2">
              Access Denied
            </h1>
            <p className="text-dark-400 mb-6">
              You don't have permission to access this resource.
            </p>
            <button
              onClick={() => window.history.back()}
              className="btn-secondary"
            >
              Go Back
            </button>
          </div>
        </div>
      );
    }
  }

  // Check required role
  if (requiredRole) {
    if (!hasRole(requiredRole)) {
      return (
        <div className="min-h-screen bg-dark-900 flex items-center justify-center">
          <div className="max-w-md w-full bg-dark-800 border border-dark-700 rounded-xl p-8 text-center">
            <h1 className="text-xl font-semibold text-dark-100 mb-2">
              Insufficient Privileges
            </h1>
            <p className="text-dark-400 mb-6">
              This page requires the {requiredRole} role.
            </p>
            <button
              onClick={() => window.history.back()}
              className="btn-secondary"
            >
              Go Back
            </button>
          </div>
        </div>
      );
    }
  }

  return <>{children}</>;
}
