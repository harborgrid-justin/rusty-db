import { useState, FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { LockClosedIcon, UserIcon, CircleStackIcon } from '@heroicons/react/24/outline';
import { Input } from '../components/common/Input';
import { Button } from '../components/common/Button';
import { useAuth } from '../hooks/useAuth';

// ============================================================================
// Login Page
// User authentication interface
// ============================================================================

export default function Login() {
  const navigate = useNavigate();
  const { login, isLoading, error } = useAuth();

  const [formData, setFormData] = useState({
    username: '',
    password: '',
    database: '',
    rememberMe: false,
  });

  const [formErrors, setFormErrors] = useState({
    username: '',
    password: '',
  });

  const validateForm = (): boolean => {
    const errors = {
      username: '',
      password: '',
    };

    if (!formData.username.trim()) {
      errors.username = 'Username is required';
    }

    if (!formData.password) {
      errors.password = 'Password is required';
    }

    setFormErrors(errors);
    return !errors.username && !errors.password;
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    if (!validateForm()) {
      return;
    }

    try {
      await login({
        username: formData.username,
        password: formData.password,
        database: formData.database,
        rememberMe: formData.rememberMe,
      });
      navigate('/');
    } catch (err) {
      // Error is handled by auth context
    }
  };

  const handleInputChange = (field: 'username' | 'password' | 'database', value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    // Clear error when user starts typing
    if (field !== 'database' && formErrors[field]) {
      setFormErrors((prev) => ({ ...prev, [field]: '' }));
    }
  };

  const handleAutoFill = () => {
    setFormData((prev) => ({
      ...prev,
      username: 'admin',
      password: 'admin',
    }));
    setFormErrors({
      username: '',
      password: '',
    });
  };

  return (
    <div className="min-h-screen bg-dark-900 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
      <motion.div
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="sm:mx-auto sm:w-full sm:max-w-md"
      >
        {/* Logo */}
        <div className="flex justify-center mb-6">
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
          </svg>
        </div>

        {/* Title */}
        <h2 className="text-center text-3xl font-bold text-dark-100">
          Welcome to RustyDB
        </h2>
        <p className="mt-2 text-center text-sm text-dark-400">
          Enterprise Database Management Platform
        </p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, delay: 0.1 }}
        className="mt-8 sm:mx-auto sm:w-full sm:max-w-md"
      >
        <div className="bg-dark-800 py-8 px-6 shadow-2xl rounded-xl border border-dark-700">
          <form className="space-y-6" onSubmit={handleSubmit}>
            {/* Error Alert */}
            {error && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="bg-danger-500/10 border border-danger-500/30 rounded-lg p-4"
              >
                <p className="text-sm text-danger-400">{error}</p>
              </motion.div>
            )}

            {/* Database Input */}
            <Input
              id="database"
              label="Database (Optional)"
              type="text"
              value={formData.database}
              onChange={(e) => handleInputChange('database', e.target.value)}
              leftIcon={<CircleStackIcon className="w-5 h-5" />}
              placeholder="Enter database name"
              disabled={isLoading}
              fullWidth
            />

            {/* Username Input */}
            <Input
              id="username"
              label="Username"
              type="text"
              autoComplete="username"
              value={formData.username}
              onChange={(e) => handleInputChange('username', e.target.value)}
              error={formErrors.username}
              leftIcon={<UserIcon className="w-5 h-5" />}
              placeholder="Enter your username"
              disabled={isLoading}
              fullWidth
              required
            />

            {/* Password Input */}
            <Input
              id="password"
              label="Password"
              type="password"
              autoComplete="current-password"
              value={formData.password}
              onChange={(e) => handleInputChange('password', e.target.value)}
              error={formErrors.password}
              leftIcon={<LockClosedIcon className="w-5 h-5" />}
              placeholder="Enter your password"
              disabled={isLoading}
              fullWidth
              required
            />

            {/* Remember Me & Forgot Password */}
            <div className="flex items-center justify-between">
              <div className="flex items-center">
                <input
                  id="remember-me"
                  name="remember-me"
                  type="checkbox"
                  checked={formData.rememberMe}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, rememberMe: e.target.checked }))
                  }
                  className="h-4 w-4 rounded border-dark-600 bg-dark-700 text-rusty-500 focus:ring-rusty-500 focus:ring-offset-dark-800"
                />
                <label
                  htmlFor="remember-me"
                  className="ml-2 block text-sm text-dark-300"
                >
                  Remember me
                </label>
              </div>

              <div className="text-sm flex items-center gap-4">
                <button
                  type="button"
                  onClick={handleAutoFill}
                  className="font-medium text-rusty-500 hover:text-rusty-400 transition-colors"
                >
                  Auto-fill Admin
                </button>
                <a
                  href="#"
                  className="font-medium text-rusty-500 hover:text-rusty-400 transition-colors"
                >
                  Forgot password?
                </a>
              </div>
            </div>

            {/* Submit Button */}
            <Button
              type="submit"
              variant="primary"
              fullWidth
              loading={isLoading}
              size="lg"
            >
              Sign in
            </Button>
          </form>

          {/* Divider */}
          <div className="mt-6">
            <div className="relative">
              <div className="absolute inset-0 flex items-center">
                <div className="w-full border-t border-dark-700" />
              </div>
              <div className="relative flex justify-center text-sm">
                <span className="px-2 bg-dark-800 text-dark-400">
                  Need help?
                </span>
              </div>
            </div>
          </div>

          {/* Help Text */}
          <div className="mt-6 text-center">
            <p className="text-xs text-dark-400">
              Contact your system administrator for access
            </p>
          </div>
        </div>
      </motion.div>

      {/* Footer */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.2 }}
        className="mt-8 text-center"
      >
        <p className="text-xs text-dark-500">
          RustyDB Enterprise &copy; {new Date().getFullYear()}. All rights reserved.
        </p>
      </motion.div>
    </div>
  );
}
