import { useState, useCallback } from 'react';
import { Outlet, Link, useLocation, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  HomeIcon,
  CommandLineIcon,
  TableCellsIcon,
  UsersIcon,
  ChartBarIcon,
  ShieldCheckIcon,
  CloudArrowUpIcon,
  ServerStackIcon,
  CogIcon,
  CpuChipIcon,
  Bars3Icon,
  XMarkIcon,
  BellIcon,
  MagnifyingGlassIcon,
  ChevronDownIcon,
  ArrowRightOnRectangleIcon,
  UserCircleIcon,
  SunIcon,
  MoonIcon,
  KeyIcon,
  CircleStackIcon,
  ClockIcon,
  ExclamationTriangleIcon,
  LockClosedIcon,
  EyeSlashIcon,
  ClipboardDocumentListIcon,
  CalendarIcon,
  ArrowPathIcon,
  SignalIcon,
  ArrowsRightLeftIcon,
  AdjustmentsHorizontalIcon,
  Square3Stack3DIcon,
  QueueListIcon,
  RectangleGroupIcon,
  CodeBracketIcon,
} from '@heroicons/react/24/outline';
import { useUIStore, useNotifications, useUnreadNotificationCount } from '../../stores/uiStore';
import { useAuth } from '../../hooks/useAuth';
import { useTheme } from '../../contexts/ThemeContext';
import clsx from 'clsx';

// Navigation structure
const navigation = [
  {
    name: 'Dashboard',
    href: '/dashboard',
    icon: HomeIcon,
  },
  {
    name: 'Query Editor',
    href: '/query',
    icon: CommandLineIcon,
  },
  {
    name: 'Schema',
    icon: TableCellsIcon,
    children: [
      { name: 'Tables', href: '/tables', icon: TableCellsIcon },
      { name: 'Indexes', href: '/indexes', icon: QueueListIcon },
      { name: 'Views', href: '/views', icon: RectangleGroupIcon },
      { name: 'Procedures', href: '/procedures', icon: CodeBracketIcon },
    ],
  },
  {
    name: 'Users & Roles',
    icon: UsersIcon,
    children: [
      { name: 'Users', href: '/users', icon: UsersIcon },
      { name: 'Roles', href: '/roles', icon: KeyIcon },
    ],
  },
  {
    name: 'Monitoring',
    icon: ChartBarIcon,
    children: [
      { name: 'Overview', href: '/monitoring', icon: ChartBarIcon },
      { name: 'Sessions', href: '/monitoring/sessions', icon: CircleStackIcon },
      { name: 'Slow Queries', href: '/monitoring/slow-queries', icon: ClockIcon },
      { name: 'Alerts', href: '/monitoring/alerts', icon: ExclamationTriangleIcon },
    ],
  },
  {
    name: 'Security',
    icon: ShieldCheckIcon,
    children: [
      { name: 'Overview', href: '/security', icon: ShieldCheckIcon },
      { name: 'Encryption', href: '/security/encryption', icon: LockClosedIcon },
      { name: 'Data Masking', href: '/security/masking', icon: EyeSlashIcon },
      { name: 'Audit Logs', href: '/security/audit', icon: ClipboardDocumentListIcon },
    ],
  },
  {
    name: 'Backup',
    icon: CloudArrowUpIcon,
    children: [
      { name: 'Backups', href: '/backup', icon: CloudArrowUpIcon },
      { name: 'Schedules', href: '/backup/schedules', icon: CalendarIcon },
      { name: 'Restore', href: '/backup/restore', icon: ArrowPathIcon },
    ],
  },
  {
    name: 'Cluster',
    icon: ServerStackIcon,
    children: [
      { name: 'Nodes', href: '/cluster', icon: ServerStackIcon },
      { name: 'Replication', href: '/cluster/replication', icon: ArrowsRightLeftIcon },
      { name: 'Failover', href: '/cluster/failover', icon: SignalIcon },
    ],
  },
  {
    name: 'Configuration',
    icon: CogIcon,
    children: [
      { name: 'General', href: '/config', icon: CogIcon },
      { name: 'Performance', href: '/config/performance', icon: AdjustmentsHorizontalIcon },
      { name: 'Security', href: '/config/security', icon: ShieldCheckIcon },
    ],
  },
  {
    name: 'Resources',
    icon: CpuChipIcon,
    children: [
      { name: 'Resource Groups', href: '/resources', icon: Square3Stack3DIcon },
      { name: 'Connection Pools', href: '/resources/pools', icon: CpuChipIcon },
    ],
  },
];

export function MainLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const { user, logout } = useAuth();
  const { resolvedTheme, toggleTheme } = useTheme();
  const { sidebar, toggleSidebar, setSidebarOpen } = useUIStore();
  const notifications = useNotifications();
  const unreadCount = useUnreadNotificationCount();

  const [expandedSections, setExpandedSections] = useState<string[]>(['Schema', 'Monitoring']);
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const [notificationsOpen, setNotificationsOpen] = useState(false);

  const toggleSection = useCallback((name: string) => {
    setExpandedSections((prev) =>
      prev.includes(name)
        ? prev.filter((s) => s !== name)
        : [...prev, name]
    );
  }, []);

  const handleLogout = useCallback(async () => {
    await logout();
    navigate('/login');
  }, [logout, navigate]);

  const isActive = (href: string) => location.pathname === href;
  const isSectionActive = (children: { href: string }[]) =>
    children.some((child) => location.pathname.startsWith(child.href));

  return (
    <div className="min-h-screen bg-transparent flex">
      {/* Sidebar Overlay (mobile) */}
      <AnimatePresence>
        {sidebar.isOpen && !sidebar.isPinned && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 backdrop-blur-sm z-40 lg:hidden"
            onClick={() => setSidebarOpen(false)}
          />
        )}
      </AnimatePresence>

      {/* Sidebar */}
      <aside
        className={clsx(
          'fixed lg:static inset-y-0 left-0 z-50 w-64 bg-dark-950/80 backdrop-blur-xl border-r border-white/5 flex flex-col transition-transform duration-300',
          sidebar.isOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0 lg:w-0 lg:overflow-hidden'
        )}
      >
        {/* Logo */}
        <div className="h-16 flex items-center justify-between px-6 border-b border-white/5">
          <Link to="/" className="flex items-center gap-3 group">
            <div className="w-8 h-8 bg-gradient-to-br from-rusty-500 to-rusty-600 rounded-lg flex items-center justify-center shadow-glow-rusty transition-transform group-hover:scale-105">
              <CircleStackIcon className="w-5 h-5 text-white" />
            </div>
            <span className="font-bold text-lg tracking-tight text-white">RustyDB</span>
          </Link>
          <button
            onClick={() => setSidebarOpen(false)}
            className="p-1.5 rounded-lg text-dark-400 hover:text-dark-100 hover:bg-dark-700 lg:hidden"
          >
            <XMarkIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto py-4 px-3 space-y-1">
          {navigation.map((item) => (
            <div key={item.name}>
              {item.children ? (
                <>
                  <button
                    onClick={() => toggleSection(item.name)}
                    className={clsx(
                      'w-full nav-item justify-between',
                      isSectionActive(item.children) && 'text-rusty-400'
                    )}
                  >
                    <div className="flex items-center gap-3">
                      <item.icon className="w-5 h-5" />
                      <span>{item.name}</span>
                    </div>
                    <ChevronDownIcon
                      className={clsx(
                        'w-4 h-4 transition-transform',
                        expandedSections.includes(item.name) && 'rotate-180'
                      )}
                    />
                  </button>
                  <AnimatePresence>
                    {expandedSections.includes(item.name) && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: 'auto', opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="overflow-hidden ml-4 mt-1 space-y-1"
                      >
                        {item.children.map((child) => (
                          <Link
                            key={child.href}
                            to={child.href}
                            className={clsx(
                              'nav-item',
                              isActive(child.href) && 'nav-item-active'
                            )}
                          >
                            <child.icon className="w-4 h-4" />
                            <span className="text-sm">{child.name}</span>
                          </Link>
                        ))}
                      </motion.div>
                    )}
                  </AnimatePresence>
                </>
              ) : (
                <Link
                  to={item.href!}
                  className={clsx(
                    'nav-item',
                    isActive(item.href!) && 'nav-item-active'
                  )}
                >
                  <item.icon className="w-5 h-5" />
                  <span>{item.name}</span>
                </Link>
              )}
            </div>
          ))}
        </nav>

        {/* Sidebar Footer */}
        <div className="p-4 border-t border-white/5">
          <div className="flex items-center gap-3 px-3 py-2 rounded-lg bg-white/5 border border-white/5">
            <div className="w-2 h-2 rounded-full bg-success-500 animate-pulse shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
            <span className="text-xs font-medium text-dark-300">System Healthy</span>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Header */}
        <header className="h-16 bg-dark-950/80 backdrop-blur-xl border-b border-white/5 flex items-center justify-between px-4 lg:px-6">
          <div className="flex items-center gap-4">
            <button
              onClick={toggleSidebar}
              className="p-2 rounded-lg text-dark-400 hover:text-dark-100 hover:bg-white/5"
            >
              <Bars3Icon className="w-5 h-5" />
            </button>

            {/* Search */}
            <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 bg-white/5 border border-white/5 rounded-lg text-dark-400 w-64 focus-within:border-primary-500/50 focus-within:ring-1 focus-within:ring-primary-500/50 transition-all">
              <MagnifyingGlassIcon className="w-4 h-4" />
              <input
                type="text"
                placeholder="Search... (Ctrl+K)"
                className="bg-transparent border-none outline-none text-sm w-full text-dark-200 placeholder-dark-500"
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            {/* Theme Toggle */}
            <button
              onClick={toggleTheme}
              className="p-2 rounded-lg text-dark-400 hover:text-dark-100 hover:bg-dark-700"
              title={`Switch to ${resolvedTheme === 'dark' ? 'light' : 'dark'} mode`}
            >
              {resolvedTheme === 'dark' ? (
                <SunIcon className="w-5 h-5" />
              ) : (
                <MoonIcon className="w-5 h-5" />
              )}
            </button>

            {/* Notifications */}
            <div className="relative">
              <button
                onClick={() => setNotificationsOpen(!notificationsOpen)}
                className="p-2 rounded-lg text-dark-400 hover:text-dark-100 hover:bg-dark-700 relative"
              >
                <BellIcon className="w-5 h-5" />
                {unreadCount > 0 && (
                  <span className="absolute top-1 right-1 w-4 h-4 bg-danger-500 rounded-full text-[10px] font-medium text-white flex items-center justify-center">
                    {unreadCount > 9 ? '9+' : unreadCount}
                  </span>
                )}
              </button>

              <AnimatePresence>
                {notificationsOpen && (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 10 }}
                    className="absolute right-0 mt-2 w-80 bg-dark-800 border border-dark-700 rounded-xl shadow-xl z-50"
                  >
                    <div className="p-4 border-b border-dark-700 flex items-center justify-between">
                      <h3 className="font-medium text-dark-100">Notifications</h3>
                      <span className="text-xs text-dark-400">{notifications.length} total</span>
                    </div>
                    <div className="max-h-80 overflow-y-auto">
                      {notifications.length === 0 ? (
                        <div className="p-8 text-center text-dark-400 text-sm">
                          No notifications
                        </div>
                      ) : (
                        notifications.slice(0, 5).map((notification) => (
                          <div
                            key={notification.id}
                            className={clsx(
                              'p-3 border-b border-dark-700 hover:bg-dark-700/50 cursor-pointer',
                              !notification.read && 'bg-dark-700/30'
                            )}
                          >
                            <p className="text-sm text-dark-200">{notification.title}</p>
                            <p className="text-xs text-dark-400 mt-1">{notification.message}</p>
                          </div>
                        ))
                      )}
                    </div>
                    {notifications.length > 5 && (
                      <div className="p-3 border-t border-dark-700 text-center">
                        <button className="text-sm text-rusty-400 hover:text-rusty-300">
                          View all notifications
                        </button>
                      </div>
                    )}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            {/* User Menu */}
            <div className="relative">
              <button
                onClick={() => setUserMenuOpen(!userMenuOpen)}
                className="flex items-center gap-2 p-1.5 rounded-lg hover:bg-dark-700"
              >
                <div className="w-8 h-8 bg-rusty-500/20 rounded-full flex items-center justify-center">
                  <UserCircleIcon className="w-5 h-5 text-rusty-400" />
                </div>
                <span className="hidden md:block text-sm text-dark-200">
                  {user?.displayName || user?.username || 'User'}
                </span>
                <ChevronDownIcon className="w-4 h-4 text-dark-400" />
              </button>

              <AnimatePresence>
                {userMenuOpen && (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 10 }}
                    className="dropdown right-0"
                  >
                    <div className="p-3 border-b border-dark-700">
                      <p className="font-medium text-dark-100">
                        {user?.displayName || user?.username}
                      </p>
                      <p className="text-sm text-dark-400">{user?.email}</p>
                    </div>
                    <div className="py-1">
                      <button className="dropdown-item w-full">
                        <UserCircleIcon className="w-4 h-4" />
                        Profile Settings
                      </button>
                      <button className="dropdown-item w-full">
                        <CogIcon className="w-4 h-4" />
                        Preferences
                      </button>
                    </div>
                    <div className="border-t border-dark-700 py-1">
                      <button
                        onClick={handleLogout}
                        className="dropdown-item w-full text-danger-400 hover:text-danger-300"
                      >
                        <ArrowRightOnRectangleIcon className="w-4 h-4" />
                        Sign Out
                      </button>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>
        </header>

        {/* Page Content */}
        <main className="flex-1 overflow-auto p-4 lg:p-6">
          <Outlet />
        </main>

        {/* Footer */}
        <footer className="h-8 bg-dark-800 border-t border-dark-700 flex items-center justify-between px-4 text-xs text-dark-400">
          <div className="flex items-center gap-4">
            <span>RustyDB v1.0.0</span>
            <span className="hidden sm:inline">•</span>
            <span className="hidden sm:inline">Connected to Server</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-success-500" />
            <span>Ready</span>
          </div>
        </footer>
      </div>
    </div>
  );
}
