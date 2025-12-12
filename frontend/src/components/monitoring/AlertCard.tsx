import { motion } from 'framer-motion';
import { formatDistanceToNow } from 'date-fns';
import type { Alert } from '@/types';

interface AlertCardProps {
  alert: Alert;
  onAcknowledge?: (id: string, note?: string) => void;
  onResolve?: (id: string, resolution?: string) => void;
  className?: string;
}

export function AlertCard({
  alert,
  onAcknowledge,
  onResolve,
  className = '',
}: AlertCardProps) {
  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical':
        return 'bg-red-900 border-red-500 text-red-100';
      case 'error':
        return 'bg-red-800 border-red-600 text-red-100';
      case 'warning':
        return 'bg-yellow-900 border-yellow-500 text-yellow-100';
      case 'info':
        return 'bg-blue-900 border-blue-500 text-blue-100';
      default:
        return 'bg-gray-800 border-gray-600 text-gray-100';
    }
  };

  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case 'critical':
        return '🔴';
      case 'error':
        return '❌';
      case 'warning':
        return '⚠️';
      case 'info':
        return 'ℹ️';
      default:
        return '•';
    }
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'performance':
        return '⚡';
      case 'security':
        return '🔒';
      case 'availability':
        return '🌐';
      case 'capacity':
        return '💾';
      case 'replication':
        return '🔄';
      case 'backup':
        return '💼';
      case 'configuration':
        return '⚙️';
      default:
        return '📌';
    }
  };

  const handleAcknowledge = () => {
    if (onAcknowledge && !alert.acknowledged) {
      onAcknowledge(alert.id);
    }
  };

  const handleResolve = () => {
    if (onResolve && !alert.resolved) {
      onResolve(alert.id);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, x: -100 }}
      className={`border-l-4 rounded-lg p-4 ${getSeverityColor(alert.severity)} ${className}`}
    >
      <div className="flex items-start justify-between">
        <div className="flex items-start space-x-3 flex-1">
          <div className="flex-shrink-0 text-2xl">
            {getSeverityIcon(alert.severity)}
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex items-center space-x-2 mb-1">
              <h3 className="text-lg font-semibold truncate">{alert.title}</h3>
              <span className="text-xs px-2 py-1 rounded-full bg-black bg-opacity-30">
                {getTypeIcon(alert.type)} {alert.type}
              </span>
            </div>

            <p className="text-sm opacity-90 mb-2">{alert.message}</p>

            <div className="flex items-center space-x-4 text-xs opacity-75">
              <span>{formatDistanceToNow(new Date(alert.timestamp), { addSuffix: true })}</span>
              <span>Source: {alert.source}</span>
              {alert.acknowledged && (
                <span className="text-green-400">
                  ✓ Acknowledged by {alert.acknowledgedBy}
                </span>
              )}
              {alert.resolved && (
                <span className="text-green-400">
                  ✓ Resolved
                </span>
              )}
            </div>

            {alert.metadata && Object.keys(alert.metadata).length > 0 && (
              <div className="mt-2 text-xs opacity-75">
                <details className="cursor-pointer">
                  <summary className="font-medium">Additional Details</summary>
                  <pre className="mt-1 p-2 bg-black bg-opacity-30 rounded overflow-x-auto">
                    {JSON.stringify(alert.metadata, null, 2)}
                  </pre>
                </details>
              </div>
            )}
          </div>
        </div>

        <div className="flex flex-col space-y-2 ml-4">
          {!alert.acknowledged && onAcknowledge && (
            <button
              onClick={handleAcknowledge}
              className="px-3 py-1 text-xs font-medium rounded bg-black bg-opacity-30 hover:bg-opacity-50 transition-colors whitespace-nowrap"
            >
              Acknowledge
            </button>
          )}

          {!alert.resolved && onResolve && (
            <button
              onClick={handleResolve}
              className="px-3 py-1 text-xs font-medium rounded bg-green-600 hover:bg-green-700 transition-colors whitespace-nowrap"
            >
              Resolve
            </button>
          )}
        </div>
      </div>
    </motion.div>
  );
}
