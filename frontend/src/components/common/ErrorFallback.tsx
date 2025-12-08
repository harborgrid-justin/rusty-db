import { ExclamationTriangleIcon, ArrowPathIcon } from '@heroicons/react/24/outline';
import type { FallbackProps } from 'react-error-boundary';

export function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
  return (
    <div className="min-h-screen bg-dark-900 flex items-center justify-center p-4">
      <div className="max-w-md w-full bg-dark-800 border border-dark-700 rounded-xl p-8 text-center">
        <div className="w-16 h-16 mx-auto mb-6 bg-danger-500/10 rounded-full flex items-center justify-center">
          <ExclamationTriangleIcon className="w-8 h-8 text-danger-500" />
        </div>

        <h1 className="text-xl font-semibold text-dark-100 mb-2">
          Something went wrong
        </h1>

        <p className="text-dark-400 mb-6">
          An unexpected error occurred. Please try again or contact support if the problem persists.
        </p>

        {import.meta.env.DEV && error && (
          <div className="mb-6 p-4 bg-dark-900 rounded-lg text-left overflow-x-auto">
            <p className="text-sm font-mono text-danger-400 mb-2">
              {error.name}: {error.message}
            </p>
            {error.stack && (
              <pre className="text-xs font-mono text-dark-500 whitespace-pre-wrap">
                {error.stack}
              </pre>
            )}
          </div>
        )}

        <div className="flex gap-3 justify-center">
          <button
            onClick={() => (window.location.href = '/')}
            className="btn-secondary"
          >
            Go Home
          </button>
          <button
            onClick={resetErrorBoundary}
            className="btn-primary flex items-center gap-2"
          >
            <ArrowPathIcon className="w-4 h-4" />
            Try Again
          </button>
        </div>
      </div>
    </div>
  );
}
