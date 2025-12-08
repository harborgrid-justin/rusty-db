import { useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  CalendarIcon,
  PlusIcon,
  ArrowPathIcon,
  PencilIcon,
  TrashIcon,
  PlayIcon,
  PauseIcon,
  ClockIcon,
  CheckCircleIcon,
  XCircleIcon,
} from '@heroicons/react/24/outline';
import { ScheduleForm } from '../components/backup/ScheduleForm';
import { useBackupSchedules } from '../hooks/useBackup';
import { backupService } from '../services/backupService';
import type { CreateScheduleConfig } from '../services/backupService';
import type { BackupSchedule } from '../types';
import { useUIStore } from '../stores/uiStore';
import { formatCronExpression, formatDate, formatRelativeTime } from '../utils/format';
import clsx from 'clsx';

// ============================================================================
// Schedule Card Component
// ============================================================================

interface ScheduleCardProps {
  schedule: BackupSchedule;
  onEdit: (schedule: BackupSchedule) => void;
  onDelete: (schedule: BackupSchedule) => void;
  onToggle: (schedule: BackupSchedule, enabled: boolean) => void;
  onTrigger: (schedule: BackupSchedule) => void;
}

function ScheduleCard({
  schedule,
  onEdit,
  onDelete,
  onToggle,
  onTrigger,
}: ScheduleCardProps) {
  const nextRunIn = schedule.nextRun
    ? new Date(schedule.nextRun).getTime() - Date.now()
    : null;
  const isOverdue = nextRunIn !== null && nextRunIn < 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={clsx(
        'card p-6 transition-colors',
        !schedule.isEnabled && 'opacity-60'
      )}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <h3 className="text-lg font-semibold text-dark-100">
              {schedule.name}
            </h3>
            <span
              className={clsx(
                'badge',
                schedule.isEnabled ? 'badge-success' : 'badge-secondary'
              )}
            >
              {schedule.isEnabled ? 'Enabled' : 'Disabled'}
            </span>
            {schedule.lastStatus && (
              <span
                className={clsx(
                  'badge',
                  schedule.lastStatus === 'completed'
                    ? 'badge-success'
                    : 'badge-danger'
                )}
              >
                Last: {schedule.lastStatus}
              </span>
            )}
          </div>

          <div className="space-y-2">
            <div className="flex items-center gap-2 text-sm text-dark-400">
              <ClockIcon className="w-4 h-4" />
              <span>{formatCronExpression(schedule.schedule)}</span>
            </div>

            {schedule.database && (
              <div className="text-sm text-dark-400">
                Database: <span className="text-dark-300">{schedule.database}</span>
              </div>
            )}

            <div className="text-sm text-dark-400">
              Type: <span className="text-dark-300 capitalize">{schedule.type}</span>
              {' • '}
              Retention: <span className="text-dark-300">{schedule.retentionDays} days</span>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => onTrigger(schedule)}
            className="btn-ghost btn-sm"
            title="Run now"
          >
            <PlayIcon className="w-4 h-4" />
          </button>
          <button
            onClick={() => onEdit(schedule)}
            className="btn-ghost btn-sm"
            title="Edit schedule"
          >
            <PencilIcon className="w-4 h-4" />
          </button>
          <button
            onClick={() => onToggle(schedule, !schedule.isEnabled)}
            className="btn-ghost btn-sm"
            title={schedule.isEnabled ? 'Disable' : 'Enable'}
          >
            {schedule.isEnabled ? (
              <PauseIcon className="w-4 h-4" />
            ) : (
              <PlayIcon className="w-4 h-4" />
            )}
          </button>
          <button
            onClick={() => onDelete(schedule)}
            className="btn-ghost btn-sm text-danger-400 hover:text-danger-300"
            title="Delete schedule"
          >
            <TrashIcon className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 pt-4 border-t border-dark-700">
        {/* Last Run */}
        <div>
          <div className="text-xs text-dark-400 mb-1">Last Run</div>
          <div className="flex items-center gap-2">
            {schedule.lastRun ? (
              <>
                {schedule.lastStatus === 'completed' ? (
                  <CheckCircleIcon className="w-4 h-4 text-success-400" />
                ) : (
                  <XCircleIcon className="w-4 h-4 text-danger-400" />
                )}
                <span className="text-sm text-dark-200">
                  {formatRelativeTime(schedule.lastRun)}
                </span>
              </>
            ) : (
              <span className="text-sm text-dark-400">Never</span>
            )}
          </div>
        </div>

        {/* Next Run */}
        <div>
          <div className="text-xs text-dark-400 mb-1">Next Run</div>
          {schedule.isEnabled && schedule.nextRun ? (
            <div className="flex items-center gap-2">
              <ClockIcon className={clsx(
                'w-4 h-4',
                isOverdue ? 'text-warning-400' : 'text-blue-400'
              )} />
              <span className={clsx(
                'text-sm',
                isOverdue ? 'text-warning-300' : 'text-dark-200'
              )}>
                {isOverdue ? 'Overdue' : formatRelativeTime(schedule.nextRun)}
              </span>
            </div>
          ) : (
            <span className="text-sm text-dark-400">
              {schedule.isEnabled ? 'Calculating...' : 'Disabled'}
            </span>
          )}
        </div>
      </div>
    </motion.div>
  );
}

// ============================================================================
// BackupSchedules Page Component
// ============================================================================

export default function BackupSchedulesPage() {
  const [showForm, setShowForm] = useState(false);
  const [editingSchedule, setEditingSchedule] = useState<BackupSchedule | undefined>();

  const { schedules, loading, error, refetch, toggleSchedule, deleteSchedule } =
    useBackupSchedules();
  const { addNotification, showConfirmDialog, hideConfirmDialog } = useUIStore();

  const handleCreateOrUpdate = useCallback(
    async (config: CreateScheduleConfig) => {
      try {
        if (editingSchedule) {
          await backupService.updateSchedule(editingSchedule.id, config);
          addNotification({
            type: 'success',
            title: 'Schedule Updated',
            message: `Schedule "${config.name}" has been updated`,
          });
        } else {
          await backupService.createSchedule(config);
          addNotification({
            type: 'success',
            title: 'Schedule Created',
            message: `Schedule "${config.name}" has been created`,
          });
        }
        setShowForm(false);
        setEditingSchedule(undefined);
        refetch();
      } catch (error) {
        console.error('Failed to save schedule:', error);
        throw error;
      }
    },
    [editingSchedule, addNotification, refetch]
  );

  const handleEdit = useCallback((schedule: BackupSchedule) => {
    setEditingSchedule(schedule);
    setShowForm(true);
  }, []);

  const handleDelete = useCallback(
    (schedule: BackupSchedule) => {
      showConfirmDialog({
        title: 'Delete Schedule',
        message: `Are you sure you want to delete "${schedule.name}"? This will not delete existing backups created by this schedule.`,
        confirmLabel: 'Delete',
        cancelLabel: 'Cancel',
        variant: 'danger',
        onConfirm: async () => {
          try {
            await deleteSchedule(schedule.id);
            hideConfirmDialog();
          } catch (error) {
            console.error('Failed to delete schedule:', error);
            hideConfirmDialog();
          }
        },
        onCancel: () => {
          hideConfirmDialog();
        },
      });
    },
    [showConfirmDialog, hideConfirmDialog, deleteSchedule]
  );

  const handleToggle = useCallback(
    async (schedule: BackupSchedule, enabled: boolean) => {
      try {
        await toggleSchedule(schedule.id, enabled);
      } catch (error) {
        console.error('Failed to toggle schedule:', error);
      }
    },
    [toggleSchedule]
  );

  const handleTrigger = useCallback(
    async (schedule: BackupSchedule) => {
      try {
        await backupService.triggerSchedule(schedule.id);
        addNotification({
          type: 'success',
          title: 'Backup Triggered',
          message: `Manual backup from schedule "${schedule.name}" has been started`,
        });
        // Optionally navigate to backups page
        // window.location.href = '/backup';
      } catch (error) {
        addNotification({
          type: 'error',
          title: 'Failed to Trigger Backup',
          message: 'Could not start manual backup',
        });
      }
    },
    [addNotification]
  );

  const handleCancel = useCallback(() => {
    setShowForm(false);
    setEditingSchedule(undefined);
  }, []);

  const activeSchedules = schedules.filter((s) => s.isEnabled);
  const inactiveSchedules = schedules.filter((s) => !s.isEnabled);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100">Backup Schedules</h1>
          <p className="text-dark-400 mt-1">
            Automate backups with scheduled tasks
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button onClick={() => refetch()} className="btn-secondary">
            <ArrowPathIcon className="w-4 h-4" />
            Refresh
          </button>
          <button
            onClick={() => {
              setEditingSchedule(undefined);
              setShowForm(true);
            }}
            className="btn-primary"
          >
            <PlusIcon className="w-5 h-5" />
            Create Schedule
          </button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card p-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
              <CalendarIcon className="w-5 h-5 text-rusty-400" />
            </div>
            <div>
              <div className="text-xs text-dark-400">Total Schedules</div>
              <div className="text-xl font-semibold text-dark-100">
                {schedules.length}
              </div>
            </div>
          </div>
        </div>

        <div className="card p-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-success-500/20 rounded-lg flex items-center justify-center">
              <PlayIcon className="w-5 h-5 text-success-400" />
            </div>
            <div>
              <div className="text-xs text-dark-400">Active</div>
              <div className="text-xl font-semibold text-dark-100">
                {activeSchedules.length}
              </div>
            </div>
          </div>
        </div>

        <div className="card p-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-dark-600/50 rounded-lg flex items-center justify-center">
              <PauseIcon className="w-5 h-5 text-dark-400" />
            </div>
            <div>
              <div className="text-xs text-dark-400">Inactive</div>
              <div className="text-xl font-semibold text-dark-100">
                {inactiveSchedules.length}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Error State */}
      {error && (
        <div className="card p-4 bg-danger-500/10 border border-danger-500/30">
          <p className="text-danger-300">{error}</p>
        </div>
      )}

      {/* Form */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6">
              <h2 className="text-xl font-semibold text-dark-100 mb-6">
                {editingSchedule ? 'Edit Schedule' : 'Create New Schedule'}
              </h2>
              <ScheduleForm
                schedule={editingSchedule}
                onSubmit={handleCreateOrUpdate}
                onCancel={handleCancel}
                databases={[]} // TODO: Fetch available databases
                loading={false}
              />
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Schedules List */}
      {loading ? (
        <div className="card p-8 text-center">
          <div className="inline-block w-8 h-8 border-4 border-dark-600 border-t-rusty-500 rounded-full animate-spin" />
          <p className="mt-4 text-dark-400">Loading schedules...</p>
        </div>
      ) : schedules.length === 0 ? (
        <div className="card p-8 text-center">
          <CalendarIcon className="w-12 h-12 text-dark-600 mx-auto" />
          <h3 className="mt-4 text-lg font-medium text-dark-300">
            No schedules configured
          </h3>
          <p className="mt-2 text-dark-400">
            Create your first backup schedule to automate backups
          </p>
          <button
            onClick={() => {
              setEditingSchedule(undefined);
              setShowForm(true);
            }}
            className="btn-primary mt-4"
          >
            <PlusIcon className="w-5 h-5" />
            Create Schedule
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Active Schedules */}
          {activeSchedules.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-dark-300 mb-3">
                Active Schedules ({activeSchedules.length})
              </h3>
              <div className="space-y-3">
                {activeSchedules.map((schedule) => (
                  <ScheduleCard
                    key={schedule.id}
                    schedule={schedule}
                    onEdit={handleEdit}
                    onDelete={handleDelete}
                    onToggle={handleToggle}
                    onTrigger={handleTrigger}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Inactive Schedules */}
          {inactiveSchedules.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-dark-300 mb-3">
                Inactive Schedules ({inactiveSchedules.length})
              </h3>
              <div className="space-y-3">
                {inactiveSchedules.map((schedule) => (
                  <ScheduleCard
                    key={schedule.id}
                    schedule={schedule}
                    onEdit={handleEdit}
                    onDelete={handleDelete}
                    onToggle={handleToggle}
                    onTrigger={handleTrigger}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
