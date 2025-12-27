import { ReactNode, useState, createContext, useContext } from 'react';
import { motion } from 'framer-motion';

// ============================================================================
// Tabs Component
// Tab navigation for organizing content
// ============================================================================

export interface Tab {
  id: string;
  label: string | ReactNode;
  icon?: ReactNode;
  disabled?: boolean;
  badge?: number | string;
}

interface TabsContextValue {
  activeTab: string;
  setActiveTab: (id: string) => void;
}

const TabsContext = createContext<TabsContextValue | undefined>(undefined);

function useTabsContext() {
  const context = useContext(TabsContext);
  if (!context) {
    throw new Error('Tab components must be used within Tabs');
  }
  return context;
}

// Main Tabs container
export interface TabsProps {
  children: ReactNode;
  defaultTab?: string;
  value?: string;
  onChange?: (tabId: string) => void;
  className?: string;
}

export function Tabs({
  children,
  defaultTab,
  value,
  onChange,
  className = '',
}: TabsProps) {
  const [internalActiveTab, setInternalActiveTab] = useState(defaultTab || '');
  const activeTab = value ?? internalActiveTab;

  const handleTabChange = (tabId: string) => {
    if (!value) {
      setInternalActiveTab(tabId);
    }
    onChange?.(tabId);
  };

  return (
    <TabsContext.Provider value={{ activeTab, setActiveTab: handleTabChange }}>
      <div className={className}>{children}</div>
    </TabsContext.Provider>
  );
}

// Tab List - renders the tab buttons
export interface TabListProps {
  tabs: Tab[];
  variant?: 'default' | 'pills' | 'underline';
  fullWidth?: boolean;
  className?: string;
}

export function TabList({
  tabs,
  variant = 'default',
  fullWidth = false,
  className = '',
}: TabListProps) {
  const { activeTab, setActiveTab } = useTabsContext();

  const variantClasses = {
    default: {
      container: 'border-b border-dark-700',
      button: 'px-4 py-2 text-sm font-medium transition-colors relative',
      active: 'text-rusty-500',
      inactive: 'text-dark-400 hover:text-dark-200',
      indicator: true,
    },
    pills: {
      container: 'bg-dark-700/50 p-1 rounded-lg',
      button: 'px-4 py-2 text-sm font-medium rounded-md transition-colors relative',
      active: 'bg-rusty-500 text-white shadow-md',
      inactive: 'text-dark-400 hover:text-dark-200 hover:bg-dark-700',
      indicator: false,
    },
    underline: {
      container: '',
      button: 'px-4 py-2 text-sm font-medium transition-colors border-b-2 border-transparent',
      active: 'text-rusty-500 border-rusty-500',
      inactive: 'text-dark-400 hover:text-dark-200',
      indicator: false,
    },
  };

  const config = variantClasses[variant];

  return (
    <div className={`${config.container} ${fullWidth ? 'w-full' : ''} ${className}`}>
      <nav className={`flex ${fullWidth ? 'w-full' : ''} gap-1`}>
        {tabs.map((tab) => {
          const isActive = activeTab === tab.id;

          return (
            <button
              key={tab.id}
              onClick={() => !tab.disabled && setActiveTab(tab.id)}
              disabled={tab.disabled}
              className={`
                ${config.button}
                ${isActive ? config.active : config.inactive}
                ${tab.disabled ? 'opacity-50 cursor-not-allowed' : ''}
                ${fullWidth ? 'flex-1' : ''}
              `}
            >
              <span className="flex items-center justify-center gap-2">
                {tab.icon}
                {tab.label}
                {tab.badge !== undefined && (
                  <span className="px-2 py-0.5 text-xs rounded-full bg-rusty-500/20 text-rusty-400">
                    {tab.badge}
                  </span>
                )}
              </span>

              {config.indicator && isActive && (
                <motion.div
                  layoutId="activeTab"
                  className="absolute bottom-0 left-0 right-0 h-0.5 bg-rusty-500"
                  transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                />
              )}
            </button>
          );
        })}
      </nav>
    </div>
  );
}

// Tab Panel - content for each tab
export interface TabPanelProps {
  tabId: string;
  children: ReactNode;
  className?: string;
  lazy?: boolean;
}

export function TabPanel({ tabId, children, className = '', lazy = false }: TabPanelProps) {
  const { activeTab } = useTabsContext();
  const isActive = activeTab === tabId;

  if (lazy && !isActive) {
    return null;
  }

  return (
    <div
      role="tabpanel"
      hidden={!isActive}
      className={`${isActive ? 'block' : 'hidden'} ${className}`}
    >
      {children}
    </div>
  );
}

// Tab Panels container
export interface TabPanelsProps {
  children: ReactNode;
  className?: string;
}

export function TabPanels({ children, className = '' }: TabPanelsProps) {
  return <div className={className}>{children}</div>;
}
