import { SelectHTMLAttributes, forwardRef, ReactNode } from 'react';
import { ChevronDownIcon, ExclamationCircleIcon } from '@heroicons/react/24/outline';

// ============================================================================
// Select Component
// Reusable dropdown select component with validation
// ============================================================================

export interface SelectOption {
  value: string | number;
  label: string;
  disabled?: boolean;
}

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  label?: string;
  error?: string;
  helperText?: string;
  options: SelectOption[];
  placeholder?: string;
  selectSize?: 'sm' | 'md' | 'lg';
  fullWidth?: boolean;
  leftIcon?: ReactNode;
}

const sizeClasses = {
  sm: 'px-3 py-1.5 text-sm',
  md: 'px-4 py-2 text-sm',
  lg: 'px-5 py-3 text-base',
};

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  (
    {
      label,
      error,
      helperText,
      options,
      placeholder,
      selectSize = 'md',
      fullWidth = false,
      leftIcon,
      className = '',
      disabled,
      ...props
    },
    ref
  ) => {
    const baseClasses = 'bg-dark-700 border rounded-lg transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-rusty-500 focus:border-transparent appearance-none';
    const stateClasses = error
      ? 'border-danger-500 focus:ring-danger-500'
      : 'border-dark-600 hover:border-dark-500';
    const disabledClasses = disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer';

    const selectClasses = [
      baseClasses,
      sizeClasses[selectSize],
      stateClasses,
      disabledClasses,
      leftIcon ? 'pl-10' : '',
      'pr-10',
      fullWidth ? 'w-full' : '',
      'text-dark-100',
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
            <div className="absolute left-3 top-1/2 -translate-y-1/2 text-dark-500 pointer-events-none">
              {leftIcon}
            </div>
          )}

          <select
            ref={ref}
            className={selectClasses}
            disabled={disabled}
            aria-invalid={!!error}
            aria-describedby={error ? `${props.id}-error` : helperText ? `${props.id}-helper` : undefined}
            {...props}
          >
            {placeholder && (
              <option value="" disabled>
                {placeholder}
              </option>
            )}
            {options.map((option) => (
              <option
                key={option.value}
                value={option.value}
                disabled={option.disabled}
              >
                {option.label}
              </option>
            ))}
          </select>

          <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
            {error ? (
              <ExclamationCircleIcon className="w-5 h-5 text-danger-500" />
            ) : (
              <ChevronDownIcon className="w-5 h-5 text-dark-500" />
            )}
          </div>
        </div>

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

Select.displayName = 'Select';

// Multi-select component (simplified version, for complex use cases consider a library)
export interface MultiSelectProps extends Omit<SelectProps, 'value' | 'onChange'> {
  value: string[];
  onChange: (value: string[]) => void;
}

export function MultiSelect({
  value,
  onChange,
  options,
  label,
  error,
  helperText,
  fullWidth = false,
  className = '',
  ...props
}: MultiSelectProps) {
  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const selectedOptions = Array.from(e.target.selectedOptions).map(opt => opt.value);
    onChange(selectedOptions);
  };

  return (
    <div className={fullWidth ? 'w-full' : ''}>
      {label && (
        <label className="block text-sm font-medium text-dark-300 mb-1.5">
          {label}
        </label>
      )}

      <select
        multiple
        value={value}
        onChange={handleChange}
        className={`bg-dark-700 border rounded-lg transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-rusty-500 focus:border-transparent px-4 py-2 text-sm ${
          error ? 'border-danger-500' : 'border-dark-600 hover:border-dark-500'
        } ${fullWidth ? 'w-full' : ''} text-dark-100 ${className}`}
        {...props}
      >
        {options.map((option) => (
          <option
            key={option.value}
            value={option.value}
            disabled={option.disabled}
            className="py-1"
          >
            {option.label}
          </option>
        ))}
      </select>

      {error && (
        <p className="mt-1.5 text-sm text-danger-500">{error}</p>
      )}

      {helperText && !error && (
        <p className="mt-1.5 text-sm text-dark-400">{helperText}</p>
      )}
    </div>
  );
}
