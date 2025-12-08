import { motion } from 'framer-motion';

interface LoadingScreenProps {
  message?: string;
}

export function LoadingScreen({ message = 'Loading...' }: LoadingScreenProps) {
  return (
    <div className="min-h-screen bg-dark-900 flex flex-col items-center justify-center">
      {/* Logo */}
      <motion.div
        initial={{ opacity: 0, scale: 0.9 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.3 }}
        className="mb-8"
      >
        <svg
          className="w-16 h-16"
          viewBox="0 0 64 64"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <rect
            x="8"
            y="8"
            width="48"
            height="48"
            rx="8"
            fill="#ed7519"
            fillOpacity="0.2"
          />
          <path
            d="M32 16C23.16 16 16 23.16 16 32C16 40.84 23.16 48 32 48C40.84 48 48 40.84 48 32C48 23.16 40.84 16 32 16ZM32 44C25.38 44 20 38.62 20 32C20 25.38 25.38 20 32 20C38.62 20 44 25.38 44 32C44 38.62 38.62 44 32 44Z"
            fill="#ed7519"
          />
          <circle cx="32" cy="32" r="6" fill="#ed7519" />
          <motion.path
            d="M32 26V22M32 42V38M38 32H42M22 32H26"
            stroke="#ed7519"
            strokeWidth="2"
            strokeLinecap="round"
            initial={{ pathLength: 0 }}
            animate={{ pathLength: 1 }}
            transition={{ duration: 1, repeat: Infinity }}
          />
        </svg>
      </motion.div>

      {/* Spinner */}
      <motion.div
        className="relative w-12 h-12 mb-4"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.2 }}
      >
        <motion.div
          className="absolute inset-0 border-4 border-dark-700 rounded-full"
        />
        <motion.div
          className="absolute inset-0 border-4 border-transparent border-t-rusty-500 rounded-full"
          animate={{ rotate: 360 }}
          transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
        />
      </motion.div>

      {/* Message */}
      <motion.p
        className="text-dark-400 text-sm"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.3 }}
      >
        {message}
      </motion.p>
    </div>
  );
}

// Inline loading spinner for components
export function LoadingSpinner({ size = 'md' }: { size?: 'sm' | 'md' | 'lg' }) {
  const sizeClasses = {
    sm: 'w-4 h-4 border-2',
    md: 'w-8 h-8 border-3',
    lg: 'w-12 h-12 border-4',
  };

  return (
    <div className={`${sizeClasses[size]} border-dark-600 border-t-rusty-500 rounded-full animate-spin`} />
  );
}

// Skeleton loader for content
export function Skeleton({ className = '' }: { className?: string }) {
  return (
    <div className={`skeleton animate-pulse ${className}`} />
  );
}

// Page loading overlay
export function PageLoader({ message }: { message?: string }) {
  return (
    <div className="absolute inset-0 bg-dark-900/80 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="flex flex-col items-center gap-4">
        <LoadingSpinner size="lg" />
        {message && (
          <p className="text-dark-300 text-sm">{message}</p>
        )}
      </div>
    </div>
  );
}
