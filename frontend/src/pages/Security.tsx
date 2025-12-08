// ============================================================================
// Security Overview Page
// Main security dashboard with status indicators and quick actions
// ============================================================================

import { useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import {
  ShieldCheckIcon,
  LockClosedIcon,
  EyeSlashIcon,
  ClipboardDocumentListIcon,
  KeyIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon,
  ClockIcon,
  ArrowPathIcon,
  ChartBarIcon,
  BellAlertIcon,
  DocumentChartBarIcon,
} from '@heroicons/react/24/outline';
import { useSecurityData, useRunSecurityScan } from '../hooks/useSecurity';
import { LoadingScreen } from '../components/common/LoadingScreen';
import clsx from 'clsx';

// ============================================================================
// Security Overview Component
// ============================================================================

export default function Security() {
  const { overview, alerts, keys, policies, isLoading, error } = useSecurityData();
  const runSecurityScan = useRunSecurityScan();
  const [selectedPeriod, setSelectedPeriod] = useState<'24h' | '7d' | '30d'>('24h');

  if (isLoading) {
    return <LoadingScreen />;
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <ExclamationTriangleIcon className="w-12 h-12 text-danger-500 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-dark-100 mb-2">Failed to load security data</h3>
          <p className="text-dark-400">{error}</p>
        </div>
      </div>
    );
  }

  const complianceScore = overview?.complianceScore || 0;
  const activeAlerts = alerts?.filter((a) => !a.resolved) || [];
  const criticalAlerts = activeAlerts.filter((a) => a.severity === 'critical');

  const handleRunScan = async () => {
    try {
      await runSecurityScan.mutateAsync();
    } catch (err) {
      console.error('Failed to run security scan:', err);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100 flex items-center gap-3">
            <ShieldCheckIcon className="w-8 h-8 text-rusty-500" />
            Security Overview
          </h1>
          <p className="text-dark-400 mt-1">
            Monitor encryption, masking policies, and audit logs
          </p>
        </div>
        <div className="flex items-center gap-3">
          <select
            value={selectedPeriod}
            onChange={(e) => setSelectedPeriod(e.target.value as '24h' | '7d' | '30d')}
            className="input-field w-32"
          >
            <option value="24h">Last 24h</option>
            <option value="7d">Last 7 days</option>
            <option value="30d">Last 30 days</option>
          </select>
          <button
            onClick={handleRunScan}
            disabled={runSecurityScan.isPending}
            className="btn-secondary"
          >
            <ArrowPathIcon
              className={clsx('w-4 h-4', runSecurityScan.isPending && 'animate-spin')}
            />
            Security Scan
          </button>
        </div>
      </div>

      {/* Critical Alerts Banner */}
      {criticalAlerts.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-danger-500/10 border border-danger-500/30 rounded-xl p-4"
        >
          <div className="flex items-start gap-3">
            <BellAlertIcon className="w-6 h-6 text-danger-500 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <h3 className="font-medium text-danger-400 mb-1">
                {criticalAlerts.length} Critical Security Alert{criticalAlerts.length > 1 ? 's' : ''}
              </h3>
              <p className="text-sm text-dark-300">
                Immediate attention required. Review and resolve these alerts.
              </p>
            </div>
            <Link to="/security/audit" className="btn-danger text-sm">
              View Alerts
            </Link>
          </div>
        </motion.div>
      )}

      {/* Compliance Score */}
      <div className="card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-dark-100">Compliance Score</h2>
          <Link to="/security/audit" className="text-sm text-rusty-400 hover:text-rusty-300">
            View Details →
          </Link>
        </div>
        <div className="flex items-center gap-6">
          <div className="relative w-32 h-32">
            <svg className="transform -rotate-90 w-32 h-32">
              <circle
                cx="64"
                cy="64"
                r="56"
                stroke="currentColor"
                strokeWidth="8"
                fill="none"
                className="text-dark-700"
              />
              <circle
                cx="64"
                cy="64"
                r="56"
                stroke="currentColor"
                strokeWidth="8"
                fill="none"
                strokeDasharray={`${2 * Math.PI * 56}`}
                strokeDashoffset={`${2 * Math.PI * 56 * (1 - complianceScore / 100)}`}
                className={clsx(
                  'transition-all duration-1000',
                  complianceScore >= 90
                    ? 'text-success-500'
                    : complianceScore >= 70
                    ? 'text-warning-500'
                    : 'text-danger-500'
                )}
                strokeLinecap="round"
              />
            </svg>
            <div className="absolute inset-0 flex items-center justify-center">
              <span className="text-2xl font-bold text-dark-100">{complianceScore}%</span>
            </div>
          </div>
          <div className="flex-1 space-y-3">
            <div>
              <div className="flex items-center justify-between text-sm mb-1">
                <span className="text-dark-300">Encryption Coverage</span>
                <span className="text-dark-100 font-medium">
                  {overview?.encryptionStatus.encryptedTables || 0} tables
                </span>
              </div>
              <div className="progress-bar">
                <div
                  className="progress-fill bg-rusty-500"
                  style={{ width: '85%' }}
                />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between text-sm mb-1">
                <span className="text-dark-300">Active Masking Policies</span>
                <span className="text-dark-100 font-medium">
                  {overview?.maskingStatus.activePolicies || 0} policies
                </span>
              </div>
              <div className="progress-bar">
                <div
                  className="progress-fill bg-rusty-500"
                  style={{ width: '92%' }}
                />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between text-sm mb-1">
                <span className="text-dark-300">Audit Coverage</span>
                <span className="text-dark-100 font-medium">Complete</span>
              </div>
              <div className="progress-bar">
                <div
                  className="progress-fill bg-success-500"
                  style={{ width: '100%' }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Status Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Encryption Status */}
        <Link to="/security/encryption" className="card hover:border-rusty-500/30 transition-colors">
          <div className="flex items-start justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-rusty-500/20 flex items-center justify-center">
              <LockClosedIcon className="w-5 h-5 text-rusty-400" />
            </div>
            {overview?.encryptionStatus.enabled ? (
              <CheckCircleIcon className="w-5 h-5 text-success-500" />
            ) : (
              <ExclamationTriangleIcon className="w-5 h-5 text-warning-500" />
            )}
          </div>
          <h3 className="text-sm font-medium text-dark-300 mb-2">Encryption</h3>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold text-dark-100">
              {overview?.encryptionStatus.activeKeys || 0}
            </span>
            <span className="text-sm text-dark-400">active keys</span>
          </div>
          <div className="mt-2 text-xs text-dark-400">
            {overview?.encryptionStatus.encryptedTables || 0} encrypted tables
          </div>
          {(overview?.encryptionStatus.expiringKeys || 0) > 0 && (
            <div className="mt-2 flex items-center gap-1 text-xs text-warning-400">
              <ClockIcon className="w-3 h-3" />
              {overview.encryptionStatus.expiringKeys} keys expiring soon
            </div>
          )}
        </Link>

        {/* Data Masking Status */}
        <Link to="/security/masking" className="card hover:border-rusty-500/30 transition-colors">
          <div className="flex items-start justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-purple-500/20 flex items-center justify-center">
              <EyeSlashIcon className="w-5 h-5 text-purple-400" />
            </div>
            <CheckCircleIcon className="w-5 h-5 text-success-500" />
          </div>
          <h3 className="text-sm font-medium text-dark-300 mb-2">Data Masking</h3>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold text-dark-100">
              {overview?.maskingStatus.activePolicies || 0}
            </span>
            <span className="text-sm text-dark-400">policies</span>
          </div>
          <div className="mt-2 text-xs text-dark-400">
            {overview?.maskingStatus.maskedColumns || 0} masked columns across{' '}
            {overview?.maskingStatus.affectedTables || 0} tables
          </div>
        </Link>

        {/* Audit Logs Status */}
        <Link to="/security/audit" className="card hover:border-rusty-500/30 transition-colors">
          <div className="flex items-start justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
              <ClipboardDocumentListIcon className="w-5 h-5 text-blue-400" />
            </div>
            <CheckCircleIcon className="w-5 h-5 text-success-500" />
          </div>
          <h3 className="text-sm font-medium text-dark-300 mb-2">Audit Logs</h3>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold text-dark-100">
              {overview?.auditStatus.eventsToday || 0}
            </span>
            <span className="text-sm text-dark-400">events today</span>
          </div>
          {(overview?.auditStatus.failedAuthentications || 0) > 0 && (
            <div className="mt-2 text-xs text-warning-400">
              {overview.auditStatus.failedAuthentications} failed authentications
            </div>
          )}
          {(overview?.auditStatus.suspiciousActivities || 0) > 0 && (
            <div className="mt-1 text-xs text-danger-400">
              {overview.auditStatus.suspiciousActivities} suspicious activities
            </div>
          )}
        </Link>

        {/* Security Alerts */}
        <div className="card">
          <div className="flex items-start justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-danger-500/20 flex items-center justify-center">
              <ExclamationTriangleIcon className="w-5 h-5 text-danger-400" />
            </div>
            {activeAlerts.length === 0 ? (
              <CheckCircleIcon className="w-5 h-5 text-success-500" />
            ) : (
              <span className="badge badge-danger">{activeAlerts.length}</span>
            )}
          </div>
          <h3 className="text-sm font-medium text-dark-300 mb-2">Active Alerts</h3>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold text-dark-100">{activeAlerts.length}</span>
            <span className="text-sm text-dark-400">unresolved</span>
          </div>
          {criticalAlerts.length > 0 && (
            <div className="mt-2 text-xs text-danger-400">
              {criticalAlerts.length} critical alerts
            </div>
          )}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <Link to="/security/encryption" className="card group hover:border-rusty-500/30">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-lg bg-rusty-500/20 flex items-center justify-center group-hover:bg-rusty-500/30 transition-colors">
              <KeyIcon className="w-6 h-6 text-rusty-400" />
            </div>
            <div className="flex-1">
              <h3 className="font-medium text-dark-100 mb-1">Manage Encryption Keys</h3>
              <p className="text-sm text-dark-400">
                Create, rotate, and monitor encryption keys
              </p>
            </div>
          </div>
        </Link>

        <Link to="/security/masking" className="card group hover:border-purple-500/30">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-lg bg-purple-500/20 flex items-center justify-center group-hover:bg-purple-500/30 transition-colors">
              <EyeSlashIcon className="w-6 h-6 text-purple-400" />
            </div>
            <div className="flex-1">
              <h3 className="font-medium text-dark-100 mb-1">Configure Data Masking</h3>
              <p className="text-sm text-dark-400">
                Set up masking policies for sensitive data
              </p>
            </div>
          </div>
        </Link>

        <Link to="/security/audit" className="card group hover:border-blue-500/30">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-lg bg-blue-500/20 flex items-center justify-center group-hover:bg-blue-500/30 transition-colors">
              <DocumentChartBarIcon className="w-6 h-6 text-blue-400" />
            </div>
            <div className="flex-1">
              <h3 className="font-medium text-dark-100 mb-1">View Audit Logs</h3>
              <p className="text-sm text-dark-400">
                Review security events and compliance reports
              </p>
            </div>
          </div>
        </Link>
      </div>

      {/* Recent Security Events */}
      {alerts && alerts.length > 0 && (
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-dark-100">Recent Security Events</h2>
            <Link to="/security/audit" className="text-sm text-rusty-400 hover:text-rusty-300">
              View All →
            </Link>
          </div>
          <div className="space-y-3">
            {alerts.slice(0, 5).map((alert) => (
              <div
                key={alert.id}
                className="flex items-start gap-3 p-3 rounded-lg bg-dark-700/50 hover:bg-dark-700 transition-colors"
              >
                <div
                  className={clsx(
                    'w-2 h-2 rounded-full mt-2',
                    alert.severity === 'critical' && 'bg-danger-500',
                    alert.severity === 'high' && 'bg-warning-500',
                    alert.severity === 'medium' && 'bg-blue-500',
                    alert.severity === 'low' && 'bg-dark-400'
                  )}
                />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <h3 className="font-medium text-dark-100 text-sm">{alert.title}</h3>
                    <span
                      className={clsx(
                        'badge text-xs',
                        alert.severity === 'critical' && 'badge-danger',
                        alert.severity === 'high' && 'badge-warning',
                        alert.severity === 'medium' && 'badge-primary',
                        alert.severity === 'low' && 'badge-secondary'
                      )}
                    >
                      {alert.severity}
                    </span>
                  </div>
                  <p className="text-sm text-dark-400 truncate">{alert.description}</p>
                  <p className="text-xs text-dark-500 mt-1">
                    {new Date(alert.timestamp).toLocaleString()}
                  </p>
                </div>
                {!alert.resolved && (
                  <span className="badge badge-warning text-xs">Active</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
