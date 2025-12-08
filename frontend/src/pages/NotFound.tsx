import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { HomeIcon, ArrowLeftIcon, MagnifyingGlassIcon } from '@heroicons/react/24/outline';
import { Button } from '../components/common/Button';

// ============================================================================
// 404 Not Found Page
// Displayed when a route doesn't exist
// ============================================================================

export default function NotFound() {
  const navigate = useNavigate();

  const suggestions = [
    { label: 'Dashboard', path: '/', icon: <HomeIcon className="w-5 h-5" /> },
    { label: 'Query Editor', path: '/query', icon: <MagnifyingGlassIcon className="w-5 h-5" /> },
    { label: 'Monitoring', path: '/monitoring', icon: <MagnifyingGlassIcon className="w-5 h-5" /> },
  ];

  return (
    <div className="min-h-screen bg-dark-900 flex items-center justify-center px-4">
      <div className="max-w-2xl w-full">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="text-center"
        >
          {/* 404 Illustration */}
          <motion.div
            initial={{ scale: 0.8, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className="mb-8"
          >
            <svg
              className="mx-auto h-64 w-64"
              viewBox="0 0 200 200"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              {/* Background circle */}
              <circle cx="100" cy="100" r="80" fill="#1f2937" opacity="0.5" />

              {/* 404 Text */}
              <text
                x="100"
                y="115"
                fontSize="48"
                fontWeight="bold"
                fill="#ed7519"
                textAnchor="middle"
              >
                404
              </text>

              {/* Database icon */}
              <circle cx="100" cy="50" r="20" stroke="#ed7519" strokeWidth="3" fill="none" />
              <line x1="80" y1="50" x2="120" y2="50" stroke="#ed7519" strokeWidth="2" />
              <line x1="100" y1="30" x2="100" y2="70" stroke="#ed7519" strokeWidth="2" />

              {/* Question marks */}
              <text x="50" y="100" fontSize="24" fill="#6b7280">?</text>
              <text x="145" y="100" fontSize="24" fill="#6b7280">?</text>
            </svg>
          </motion.div>

          {/* Error Message */}
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.2 }}
          >
            <h1 className="text-4xl font-bold text-dark-100 mb-4">
              Page Not Found
            </h1>
            <p className="text-lg text-dark-300 mb-2">
              Oops! The page you're looking for doesn't exist.
            </p>
            <p className="text-dark-400 mb-8">
              It might have been moved, deleted, or the URL might be incorrect.
            </p>
          </motion.div>

          {/* Action Buttons */}
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.3 }}
            className="flex flex-col sm:flex-row gap-4 justify-center mb-12"
          >
            <Button
              variant="primary"
              size="lg"
              leftIcon={<HomeIcon className="w-5 h-5" />}
              onClick={() => navigate('/')}
            >
              Go to Dashboard
            </Button>
            <Button
              variant="secondary"
              size="lg"
              leftIcon={<ArrowLeftIcon className="w-5 h-5" />}
              onClick={() => navigate(-1)}
            >
              Go Back
            </Button>
          </motion.div>

          {/* Suggestions */}
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.4 }}
            className="bg-dark-800 border border-dark-700 rounded-xl p-6"
          >
            <h2 className="text-lg font-semibold text-dark-100 mb-4">
              Try these pages instead:
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              {suggestions.map((suggestion, index) => (
                <motion.button
                  key={suggestion.path}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.3, delay: 0.5 + index * 0.1 }}
                  onClick={() => navigate(suggestion.path)}
                  className="flex items-center gap-3 p-4 rounded-lg bg-dark-700 hover:bg-dark-600 border border-dark-600 hover:border-rusty-500/50 transition-all group"
                >
                  <div className="text-dark-400 group-hover:text-rusty-500 transition-colors">
                    {suggestion.icon}
                  </div>
                  <span className="text-dark-200 group-hover:text-dark-100 font-medium">
                    {suggestion.label}
                  </span>
                </motion.button>
              ))}
            </div>
          </motion.div>

          {/* Help Text */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.7 }}
            className="mt-8"
          >
            <p className="text-sm text-dark-400">
              If you believe this is an error, please contact your system administrator.
            </p>
          </motion.div>
        </motion.div>
      </div>
    </div>
  );
}
