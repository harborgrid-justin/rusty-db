import { ReactNode, HTMLAttributes } from 'react';
import { motion } from 'framer-motion';

// ============================================================================
// Card Component
// Reusable card container for content grouping
// ============================================================================

export interface CardProps extends Omit<HTMLAttributes<HTMLDivElement>, 'className'> {
  children: ReactNode;
  className?: string;
  variant?: 'default' | 'bordered' | 'elevated' | 'flat';
  padding?: 'none' | 'sm' | 'md' | 'lg' | 'xl';
  hoverable?: boolean;
  animate?: boolean;
}

const variantClasses = {
  default: 'bg-dark-900/40 backdrop-blur-md border border-white/5 shadow-card',
  bordered: 'bg-dark-900/40 backdrop-blur-md border border-white/10',
  elevated: 'bg-dark-900/60 backdrop-blur-xl border border-white/5 shadow-card-hover',
  flat: 'bg-transparent border border-transparent',
};

const paddingClasses = {
  none: 'p-0',
  sm: 'p-3',
  md: 'p-4',
  lg: 'p-6',
  xl: 'p-8',
};

export function Card({
  children,
  className = '',
  variant = 'default',
  padding = 'md',
  hoverable = false,
  animate = false,
  ...props
}: CardProps) {
  const classes = [
    'rounded-xl',
    variantClasses[variant],
    paddingClasses[padding],
    hoverable ? 'transition-transform hover:scale-[1.01] cursor-pointer' : '',
    className,
  ].filter(Boolean).join(' ');

  if (animate) {
    return (
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className={classes}
        {...props}
      >
        {children}
      </motion.div>
    );
  }

  return (
    <div className={classes} {...props}>
      {children}
    </div>
  );
}

// Card Header
export interface CardHeaderProps {
  title: string | ReactNode;
  subtitle?: string | ReactNode;
  action?: ReactNode;
  className?: string;
}

export function CardHeader({ title, subtitle, action, className = '' }: CardHeaderProps) {
  return (
    <div className={`flex items-start justify-between mb-4 ${className}`}>
      <div>
        {typeof title === 'string' ? (
          <h3 className="text-lg font-semibold text-dark-100">{title}</h3>
        ) : (
          title
        )}
        {subtitle && (
          typeof subtitle === 'string' ? (
            <p className="text-sm text-dark-400 mt-1">{subtitle}</p>
          ) : (
            subtitle
          )
        )}
      </div>
      {action && <div className="flex-shrink-0 ml-4">{action}</div>}
    </div>
  );
}

// Card Body
export interface CardBodyProps {
  children: ReactNode;
  className?: string;
}

export function CardBody({ children, className = '' }: CardBodyProps) {
  return <div className={className}>{children}</div>;
}

// Card Footer
export interface CardFooterProps {
  children: ReactNode;
  className?: string;
  divider?: boolean;
}

export function CardFooter({ children, className = '', divider = true }: CardFooterProps) {
  return (
    <div className={`${divider ? 'border-t border-dark-700 pt-4' : ''} mt-4 ${className}`}>
      {children}
    </div>
  );
}

// Stat Card for metrics display
export interface StatCardProps {
  label: string;
  value: string | number;
  trend?: {
    value: number;
    direction: 'up' | 'down';
  };
  icon?: ReactNode;
  className?: string;
}

export function StatCard({ label, value, trend, icon, className = '' }: StatCardProps) {
  return (
    <Card className={className} padding="lg" hoverable>
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm text-dark-400 mb-1">{label}</p>
          <p className="text-2xl font-bold text-dark-100">{value}</p>
          {trend && (
            <div className={`flex items-center mt-2 text-sm ${
              trend.direction === 'up' ? 'text-success-500' : 'text-danger-500'
            }`}>
              {trend.direction === 'up' ? '↑' : '↓'} {Math.abs(trend.value)}%
            </div>
          )}
        </div>
        {icon && (
          <div className="p-3 bg-rusty-500/10 rounded-lg text-rusty-500">
            {icon}
          </div>
        )}
      </div>
    </Card>
  );
}
