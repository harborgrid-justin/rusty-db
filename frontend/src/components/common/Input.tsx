import { InputHTMLAttributes, forwardRef, ReactNode, useState } from 'react';
import { EyeIcon, EyeSlashIcon, ExclamationCircleIcon } from '@heroicons/react/24/outline';

// ============================================================================
// Input Component
// Reusable input field with validation states and variants
// ============================================================================

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label?: string;
  error?: string;
  helperText?: string;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  inputSize?: 'sm' | 'md' | 'lg';
  fullWidth?: boolean;
}

const sizeClasses = {
  sm: 'px-3 py-1.5 text-sm',
  md: 'px-4 py-2 text-sm',
  lg: 'px-5 py-3 text-base',
};

export const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    {
      label,
      error,
      helperText,
      leftIcon,
      rightIcon,
      inputSize = 'md',
      fullWidth = false,
      className = '',
      type = 'text',
      disabled,
      ...props
    },
    ref
  ) => {
    const [showPassword, setShowPassword] = useState(false);
    const inputType = type === 'password' && showPassword ? 'text' : type;

    const baseClasses = 'bg-dark-700 border rounded-lg transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-rusty-500 focus:border-transparent';
    const stateClasses = error
      ? 'border-danger-500 focus:ring-danger-500'
      : 'border-dark-600 hover:border-dark-500';
    const disabledClasses = disabled ? 'opacity-50 cursor-not-allowed' : '';

    const inputClasses = [
      baseClasses,
      sizeClasses[inputSize],
      stateClasses,
      disabledClasses,
      leftIcon ? 'pl-10' : '',
      rightIcon || type === 'password' ? 'pr-10' : '',
      fullWidth ? 'w-full' : '',
      'text-dark-100 placeholder-dark-500',
      className,
    ].filter(Boolean).join(' ');

    return (
      <div className={fullWidth ? 'w-full' : ''}>
        {label && (
          <label className="block text-sm font-medium text-dark-300 mb-1.5">
            {label}
          </label>
        )}

        <div className="relative">
          {leftIcon && (
            <div className="absolute left-3 top-1/2 -translate-y-1/2 text-dark-500">
              {leftIcon}
            </div>
          )}

          <input
            ref={ref}
            type={inputType}
            className={inputClasses}
            disabled={disabled}
            aria-invalid={!!error}
            aria-describedby={error ? `${props.id}-error` : helperText ? `${props.id}-helper` : undefined}
            {...props}
          />

          {type === 'password' && (
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-dark-500 hover:text-dark-300 transition-colors"
              tabIndex={-1}
            >
              {showPassword ? (
                <EyeSlashIcon className="w-5 h-5" />
              ) : (
                <EyeIcon className="w-5 h-5" />
              )}
            </button>
          )}

          {rightIcon && type !== 'password' && (
            <div className="absolute right-3 top-1/2 -translate-y-1/2 text-dark-500">
              {rightIcon}
            </div>
          )}

          {error && (
            <div className="absolute right-3 top-1/2 -translate-y-1/2 text-danger-500">
              <ExclamationCircleIcon className="w-5 h-5" />
            </div>
          )}
        </div>

        {error && (
          <p id={`${props.id}-error`} className="mt-1.5 text-sm text-danger-500 flex items-center gap-1">
            {error}
          </p>
        )}

        {helperText && !error && (
          <p id={`${props.id}-helper`} className="mt-1.5 text-sm text-dark-400">
            {helperText}
          </p>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';

// Textarea variant
export interface TextareaProps extends Omit<InputHTMLAttributes<HTMLTextAreaElement>, 'size'> {
  label?: string;
  error?: string;
  helperText?: string;
  rows?: number;
  fullWidth?: boolean;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  (
    {
      label,
      error,
      helperText,
      rows = 4,
      fullWidth = false,
      className = '',
      disabled,
      ...props
    },
    ref
  ) => {
    const baseClasses = 'bg-dark-700 border rounded-lg transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-rusty-500 focus:border-transparent px-4 py-2 text-sm';
    const stateClasses = error
      ? 'border-danger-500 focus:ring-danger-500'
      : 'border-dark-600 hover:border-dark-500';
    const disabledClasses = disabled ? 'opacity-50 cursor-not-allowed' : '';

    const textareaClasses = [
      baseClasses,
      stateClasses,
      disabledClasses,
      fullWidth ? 'w-full' : '',
      'text-dark-100 placeholder-dark-500 resize-vertical',
      className,
    ].filter(Boolean).join(' ');

    return (
      <div className={fullWidth ? 'w-full' : ''}>
        {label && (
          <label className="block text-sm font-medium text-dark-300 mb-1.5">
            {label}
          </label>
        )}

        <textarea
          ref={ref}
          rows={rows}
          className={textareaClasses}
          disabled={disabled}
          aria-invalid={!!error}
          aria-describedby={error ? `${props.id}-error` : helperText ? `${props.id}-helper` : undefined}
          {...props}
        />

        {error && (
          <p id={`${props.id}-error`} className="mt-1.5 text-sm text-danger-500">
            {error}
          </p>
        )}

        {helperText && !error && (
          <p id={`${props.id}-helper`} className="mt-1.5 text-sm text-dark-400">
            {helperText}
          </p>
        )}
      </div>
    );
  }
);

Textarea.displayName = 'Textarea';
