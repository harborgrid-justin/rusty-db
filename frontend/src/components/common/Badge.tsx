import { ReactNode } from 'react';

// ============================================================================
// Badge Component
// Status indicators and labels
// ============================================================================

export type BadgeVariant =
  | 'default'
  | 'primary'
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'neutral';

export type BadgeSize = 'sm' | 'md' | 'lg';

export interface BadgeProps {
  children: ReactNode;
  variant?: BadgeVariant;
  size?: BadgeSize;
  dot?: boolean;
  icon?: ReactNode;
  className?: string;
  onClick?: () => void;
}

const variantClasses: Record<BadgeVariant, string> = {
  default: 'bg-dark-700 text-dark-200 border-dark-600',
  primary: 'bg-rusty-500/10 text-rusty-400 border-rusty-500/30',
  success: 'bg-success-500/10 text-success-400 border-success-500/30',
  warning: 'bg-warning-500/10 text-warning-400 border-warning-500/30',
  danger: 'bg-danger-500/10 text-danger-400 border-danger-500/30',
  info: 'bg-info-500/10 text-info-400 border-info-500/30',
  neutral: 'bg-dark-600 text-dark-300 border-dark-500',
};

const sizeClasses: Record<BadgeSize, string> = {
  sm: 'px-2 py-0.5 text-xs',
  md: 'px-2.5 py-1 text-sm',
  lg: 'px-3 py-1.5 text-base',
};

const dotVariantClasses: Record<BadgeVariant, string> = {
  default: 'bg-dark-400',
  primary: 'bg-rusty-500',
  success: 'bg-success-500',
  warning: 'bg-warning-500',
  danger: 'bg-danger-500',
  info: 'bg-info-500',
  neutral: 'bg-dark-400',
};

export function Badge({
  children,
  variant = 'default',
  size = 'md',
  dot = false,
  icon,
  className = '',
  onClick,
}: BadgeProps) {
  const baseClasses = 'inline-flex items-center gap-1.5 font-medium rounded-full border';
  const clickableClasses = onClick ? 'cursor-pointer hover:opacity-80 transition-opacity' : '';

  const classes = [
    baseClasses,
    variantClasses[variant],
    sizeClasses[size],
    clickableClasses,
    className,
  ].filter(Boolean).join(' ');

  return (
    <span className={classes} onClick={onClick}>
      {dot && (
        <span className={`w-2 h-2 rounded-full ${dotVariantClasses[variant]}`} />
      )}
      {icon && <span className="flex-shrink-0">{icon}</span>}
      {children}
    </span>
  );
}

// Status badge for common status values
export interface StatusBadgeProps {
  status: 'active' | 'inactive' | 'pending' | 'error' | 'success' | 'warning';
  label?: string;
  size?: BadgeSize;
}

export function StatusBadge({ status, label, size = 'md' }: StatusBadgeProps) {
  const statusConfig = {
    active: { variant: 'success' as BadgeVariant, label: label || 'Active' },
    inactive: { variant: 'neutral' as BadgeVariant, label: label || 'Inactive' },
    pending: { variant: 'warning' as BadgeVariant, label: label || 'Pending' },
    error: { variant: 'danger' as BadgeVariant, label: label || 'Error' },
    success: { variant: 'success' as BadgeVariant, label: label || 'Success' },
    warning: { variant: 'warning' as BadgeVariant, label: label || 'Warning' },
  };

  const config = statusConfig[status];

  return (
    <Badge variant={config.variant} size={size} dot>
      {config.label}
    </Badge>
  );
}

// Health status badge
export interface HealthBadgeProps {
  health: 'healthy' | 'degraded' | 'unhealthy' | 'critical';
  size?: BadgeSize;
}

export function HealthBadge({ health, size = 'md' }: HealthBadgeProps) {
  const healthConfig = {
    healthy: { variant: 'success' as BadgeVariant, label: 'Healthy' },
    degraded: { variant: 'warning' as BadgeVariant, label: 'Degraded' },
    unhealthy: { variant: 'danger' as BadgeVariant, label: 'Unhealthy' },
    critical: { variant: 'danger' as BadgeVariant, label: 'Critical' },
  };

  const config = healthConfig[health];

  return (
    <Badge variant={config.variant} size={size} dot>
      {config.label}
    </Badge>
  );
}
