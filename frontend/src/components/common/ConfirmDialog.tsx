import { ReactNode } from 'react';
import { ExclamationTriangleIcon, InformationCircleIcon, ExclamationCircleIcon } from '@heroicons/react/24/outline';
import { Modal } from './Modal';
import { Button } from './Button';

// ============================================================================
// Confirm Dialog Component
// Specialized modal for confirmations and destructive actions
// ============================================================================

export type ConfirmDialogVariant = 'danger' | 'warning' | 'info';

export interface ConfirmDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
  title: string;
  message: string | ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: ConfirmDialogVariant;
  loading?: boolean;
  showIcon?: boolean;
}

const variantConfig: Record<ConfirmDialogVariant, {
  icon: ReactNode;
  iconBg: string;
  iconColor: string;
  buttonVariant: 'danger' | 'warning' | 'primary';
}> = {
  danger: {
    icon: <ExclamationTriangleIcon className="w-6 h-6" />,
    iconBg: 'bg-danger-500/10',
    iconColor: 'text-danger-500',
    buttonVariant: 'danger',
  },
  warning: {
    icon: <ExclamationCircleIcon className="w-6 h-6" />,
    iconBg: 'bg-warning-500/10',
    iconColor: 'text-warning-500',
    buttonVariant: 'warning',
  },
  info: {
    icon: <InformationCircleIcon className="w-6 h-6" />,
    iconBg: 'bg-info-500/10',
    iconColor: 'text-info-500',
    buttonVariant: 'primary',
  },
};

export function ConfirmDialog({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  variant = 'danger',
  loading = false,
  showIcon = true,
}: ConfirmDialogProps) {
  const config = variantConfig[variant];

  const handleConfirm = async () => {
    await onConfirm();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="sm"
      showCloseButton={false}
    >
      <div className="text-center">
        {showIcon && (
          <div className={`mx-auto flex items-center justify-center h-12 w-12 rounded-full ${config.iconBg} ${config.iconColor} mb-4`}>
            {config.icon}
          </div>
        )}

        <h3 className="text-lg font-semibold text-dark-100 mb-2">
          {title}
        </h3>

        <div className="text-sm text-dark-300 mb-6">
          {typeof message === 'string' ? <p>{message}</p> : message}
        </div>

        <div className="flex gap-3 justify-center">
          <Button
            variant="ghost"
            onClick={onClose}
            disabled={loading}
          >
            {cancelLabel}
          </Button>
          <Button
            variant={config.buttonVariant}
            onClick={handleConfirm}
            loading={loading}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

// Specialized delete confirmation dialog
export interface DeleteConfirmDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
  itemName: string;
  itemType?: string;
  loading?: boolean;
  additionalWarning?: string;
}

export function DeleteConfirmDialog({
  isOpen,
  onClose,
  onConfirm,
  itemName,
  itemType = 'item',
  loading = false,
  additionalWarning,
}: DeleteConfirmDialogProps) {
  return (
    <ConfirmDialog
      isOpen={isOpen}
      onClose={onClose}
      onConfirm={onConfirm}
      title={`Delete ${itemType}`}
      message={
        <div className="space-y-3">
          <p>
            Are you sure you want to delete <span className="font-semibold text-dark-100">{itemName}</span>?
          </p>
          {additionalWarning && (
            <p className="text-danger-400 font-medium">
              {additionalWarning}
            </p>
          )}
          <p className="text-dark-400">
            This action cannot be undone.
          </p>
        </div>
      }
      confirmLabel="Delete"
      cancelLabel="Cancel"
      variant="danger"
      loading={loading}
    />
  );
}

// Specialized discard changes dialog
export interface DiscardChangesDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
}

export function DiscardChangesDialog({
  isOpen,
  onClose,
  onConfirm,
  loading = false,
}: DiscardChangesDialogProps) {
  return (
    <ConfirmDialog
      isOpen={isOpen}
      onClose={onClose}
      onConfirm={onConfirm}
      title="Discard changes?"
      message="You have unsaved changes. Are you sure you want to discard them?"
      confirmLabel="Discard"
      cancelLabel="Keep editing"
      variant="warning"
      loading={loading}
    />
  );
}
