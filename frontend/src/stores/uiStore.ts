import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { SidebarState, TabState, NotificationItem, ConfirmDialogState } from '../types';
import { nanoid } from 'nanoid';

// ============================================================================
// UI Store State Types
// ============================================================================

interface UIState {
  // Sidebar
  sidebar: SidebarState;
  setSidebarOpen: (isOpen: boolean) => void;
  setSidebarPinned: (isPinned: boolean) => void;
  setActiveSection: (section: string) => void;
  toggleSidebar: () => void;

  // Tabs
  tabs: TabState[];
  activeTabId: string | null;
  addTab: (tab: Omit<TabState, 'id'>) => string;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTab: (id: string, updates: Partial<TabState>) => void;
  closeAllTabs: () => void;
  closeOtherTabs: (id: string) => void;

  // Notifications
  notifications: NotificationItem[];
  addNotification: (notification: Omit<NotificationItem, 'id' | 'timestamp' | 'read'>) => void;
  markNotificationRead: (id: string) => void;
  markAllNotificationsRead: () => void;
  removeNotification: (id: string) => void;
  clearNotifications: () => void;

  // Confirm Dialog
  confirmDialog: ConfirmDialogState | null;
  showConfirmDialog: (config: Omit<ConfirmDialogState, 'isOpen'>) => void;
  hideConfirmDialog: () => void;

  // Global Loading
  globalLoading: boolean;
  loadingMessage: string | null;
  setGlobalLoading: (loading: boolean, message?: string) => void;

  // Command Palette
  commandPaletteOpen: boolean;
  setCommandPaletteOpen: (open: boolean) => void;
  toggleCommandPalette: () => void;

  // Quick Actions Panel
  quickActionsOpen: boolean;
  setQuickActionsOpen: (open: boolean) => void;

  // Preferences
  preferences: {
    compactMode: boolean;
    showLineNumbers: boolean;
    wordWrap: boolean;
    fontSize: number;
    tabSize: number;
  };
  updatePreferences: (updates: Partial<UIState['preferences']>) => void;
}

// ============================================================================
// UI Store
// ============================================================================

export const useUIStore = create<UIState>()(
  persist(
    (set, get) => ({
      // Sidebar
      sidebar: {
        isOpen: true,
        isPinned: true,
        activeSection: 'dashboard',
      },
      setSidebarOpen: (isOpen) =>
        set((state) => ({
          sidebar: { ...state.sidebar, isOpen },
        })),
      setSidebarPinned: (isPinned) =>
        set((state) => ({
          sidebar: { ...state.sidebar, isPinned },
        })),
      setActiveSection: (section) =>
        set((state) => ({
          sidebar: { ...state.sidebar, activeSection: section },
        })),
      toggleSidebar: () =>
        set((state) => ({
          sidebar: { ...state.sidebar, isOpen: !state.sidebar.isOpen },
        })),

      // Tabs
      tabs: [],
      activeTabId: null,
      addTab: (tab) => {
        const id = nanoid();
        set((state) => ({
          tabs: [...state.tabs, { ...tab, id, isDirty: false }],
          activeTabId: id,
        }));
        return id;
      },
      removeTab: (id) => {
        const { tabs, activeTabId } = get();
        const newTabs = tabs.filter((t) => t.id !== id);
        let newActiveId = activeTabId;

        if (activeTabId === id && newTabs.length > 0) {
          const currentIndex = tabs.findIndex((t) => t.id === id);
          newActiveId = newTabs[Math.min(currentIndex, newTabs.length - 1)]?.id || null;
        } else if (newTabs.length === 0) {
          newActiveId = null;
        }

        set({ tabs: newTabs, activeTabId: newActiveId });
      },
      setActiveTab: (id) => set({ activeTabId: id }),
      updateTab: (id, updates) =>
        set((state) => ({
          tabs: state.tabs.map((t) =>
            t.id === id ? { ...t, ...updates } : t
          ),
        })),
      closeAllTabs: () => set({ tabs: [], activeTabId: null }),
      closeOtherTabs: (id) =>
        set((state) => ({
          tabs: state.tabs.filter((t) => t.id === id),
          activeTabId: id,
        })),

      // Notifications
      notifications: [],
      addNotification: (notification) =>
        set((state) => ({
          notifications: [
            {
              ...notification,
              id: nanoid(),
              timestamp: new Date().toISOString(),
              read: false,
            },
            ...state.notifications,
          ].slice(0, 100), // Keep max 100 notifications
        })),
      markNotificationRead: (id) =>
        set((state) => ({
          notifications: state.notifications.map((n) =>
            n.id === id ? { ...n, read: true } : n
          ),
        })),
      markAllNotificationsRead: () =>
        set((state) => ({
          notifications: state.notifications.map((n) => ({ ...n, read: true })),
        })),
      removeNotification: (id) =>
        set((state) => ({
          notifications: state.notifications.filter((n) => n.id !== id),
        })),
      clearNotifications: () => set({ notifications: [] }),

      // Confirm Dialog
      confirmDialog: null,
      showConfirmDialog: (config) =>
        set({
          confirmDialog: { ...config, isOpen: true },
        }),
      hideConfirmDialog: () => set({ confirmDialog: null }),

      // Global Loading
      globalLoading: false,
      loadingMessage: null,
      setGlobalLoading: (loading, message = null) =>
        set({
          globalLoading: loading,
          loadingMessage: loading ? message : null,
        }),

      // Command Palette
      commandPaletteOpen: false,
      setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
      toggleCommandPalette: () =>
        set((state) => ({
          commandPaletteOpen: !state.commandPaletteOpen,
        })),

      // Quick Actions Panel
      quickActionsOpen: false,
      setQuickActionsOpen: (open) => set({ quickActionsOpen: open }),

      // Preferences
      preferences: {
        compactMode: false,
        showLineNumbers: true,
        wordWrap: false,
        fontSize: 14,
        tabSize: 2,
      },
      updatePreferences: (updates) =>
        set((state) => ({
          preferences: { ...state.preferences, ...updates },
        })),
    }),
    {
      name: 'rustydb-ui-storage',
      partialize: (state) => ({
        sidebar: state.sidebar,
        preferences: state.preferences,
      }),
    }
  )
);

// ============================================================================
// Selector Hooks
// ============================================================================

export const useSidebar = () => useUIStore((state) => state.sidebar);
export const useTabs = () => useUIStore((state) => state.tabs);
export const useActiveTab = () => {
  const tabs = useUIStore((state) => state.tabs);
  const activeTabId = useUIStore((state) => state.activeTabId);
  return tabs.find((t) => t.id === activeTabId) || null;
};
export const useNotifications = () => useUIStore((state) => state.notifications);
export const useUnreadNotificationCount = () =>
  useUIStore((state) => state.notifications.filter((n) => !n.read).length);
export const usePreferences = () => useUIStore((state) => state.preferences);
