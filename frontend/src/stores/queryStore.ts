import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface QueryResult {
  columns: string[];
  rows: any[][];
  rowCount: number;
  executionTime: number;
  affected?: number;
}

export interface QueryTab {
  id: string;
  title: string;
  sql: string;
  result?: QueryResult;
  error?: string;
  isExecuting: boolean;
  isDirty: boolean;
  queryId?: string;
}

export interface QueryHistoryItem {
  id: string;
  sql: string;
  timestamp: Date;
  executionTime: number;
  rowCount?: number;
  error?: string;
}

export interface SavedQuery {
  id: string;
  name: string;
  sql: string;
  description?: string;
  createdAt: Date;
  updatedAt: Date;
  tags?: string[];
}

export interface ExplainPlan {
  nodes: PlanNode[];
  totalCost: number;
  estimatedRows: number;
}

export interface PlanNode {
  id: string;
  type: string;
  operation: string;
  table?: string;
  index?: string;
  cost: number;
  rows: number;
  actualRows?: number;
  time?: number;
  children: PlanNode[];
  details?: Record<string, any>;
  warning?: string;
}

export interface EditorPreferences {
  theme: 'vs-dark' | 'vs-light';
  fontSize: number;
  lineNumbers: boolean;
  minimap: boolean;
  wordWrap: boolean;
  formatOnSave: boolean;
  autoComplete: boolean;
}

interface QueryState {
  // Tabs
  tabs: QueryTab[];
  activeTabId: string | null;

  // History
  queryHistory: QueryHistoryItem[];

  // Saved Queries
  savedQueries: SavedQuery[];

  // Explain Plan
  explainPlan: ExplainPlan | null;

  // Preferences
  preferences: EditorPreferences;

  // Actions - Tabs
  addTab: () => string;
  closeTab: (tabId: string) => void;
  setActiveTab: (tabId: string) => void;
  updateTab: (tabId: string, updates: Partial<QueryTab>) => void;
  reorderTabs: (fromIndex: number, toIndex: number) => void;

  // Actions - Query Execution
  setTabExecuting: (tabId: string, isExecuting: boolean, queryId?: string) => void;
  setTabResult: (tabId: string, result: QueryResult) => void;
  setTabError: (tabId: string, error: string) => void;

  // Actions - History
  addToHistory: (item: Omit<QueryHistoryItem, 'id' | 'timestamp'>) => void;
  clearHistory: () => void;

  // Actions - Saved Queries
  setSavedQueries: (queries: SavedQuery[]) => void;
  addSavedQuery: (query: SavedQuery) => void;
  updateSavedQuery: (id: string, updates: Partial<SavedQuery>) => void;
  removeSavedQuery: (id: string) => void;

  // Actions - Explain Plan
  setExplainPlan: (plan: ExplainPlan | null) => void;

  // Actions - Preferences
  updatePreferences: (updates: Partial<EditorPreferences>) => void;
}

const generateId = () => `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

export const useQueryStore = create<QueryState>()(
  persist(
    (set, get) => ({
      // Initial State
      tabs: [],
      activeTabId: null,
      queryHistory: [],
      savedQueries: [],
      explainPlan: null,
      preferences: {
        theme: 'vs-dark',
        fontSize: 14,
        lineNumbers: true,
        minimap: true,
        wordWrap: false,
        formatOnSave: false,
        autoComplete: true,
      },

      // Tab Actions
      addTab: () => {
        const id = generateId();
        const newTab: QueryTab = {
          id,
          title: `Query ${get().tabs.length + 1}`,
          sql: '',
          isExecuting: false,
          isDirty: false,
        };

        set((state) => ({
          tabs: [...state.tabs, newTab],
          activeTabId: id,
        }));

        return id;
      },

      closeTab: (tabId: string) => {
        set((state) => {
          const newTabs = state.tabs.filter((tab) => tab.id !== tabId);
          let newActiveTabId = state.activeTabId;

          if (state.activeTabId === tabId) {
            const closedIndex = state.tabs.findIndex((tab) => tab.id === tabId);
            if (newTabs.length > 0) {
              const newIndex = Math.min(closedIndex, newTabs.length - 1);
              newActiveTabId = newTabs[newIndex].id;
            } else {
              newActiveTabId = null;
            }
          }

          return {
            tabs: newTabs,
            activeTabId: newActiveTabId,
          };
        });
      },

      setActiveTab: (tabId: string) => {
        set({ activeTabId: tabId });
      },

      updateTab: (tabId: string, updates: Partial<QueryTab>) => {
        set((state) => ({
          tabs: state.tabs.map((tab) =>
            tab.id === tabId
              ? { ...tab, ...updates, isDirty: updates.sql !== undefined ? true : tab.isDirty }
              : tab
          ),
        }));
      },

      reorderTabs: (fromIndex: number, toIndex: number) => {
        set((state) => {
          const newTabs = [...state.tabs];
          const [removed] = newTabs.splice(fromIndex, 1);
          newTabs.splice(toIndex, 0, removed);
          return { tabs: newTabs };
        });
      },

      // Query Execution Actions
      setTabExecuting: (tabId: string, isExecuting: boolean, queryId?: string) => {
        set((state) => ({
          tabs: state.tabs.map((tab) =>
            tab.id === tabId
              ? { ...tab, isExecuting, queryId, error: undefined }
              : tab
          ),
        }));
      },

      setTabResult: (tabId: string, result: QueryResult) => {
        set((state) => ({
          tabs: state.tabs.map((tab) =>
            tab.id === tabId
              ? { ...tab, result, error: undefined, isExecuting: false, isDirty: false }
              : tab
          ),
        }));
      },

      setTabError: (tabId: string, error: string) => {
        set((state) => ({
          tabs: state.tabs.map((tab) =>
            tab.id === tabId
              ? { ...tab, error, result: undefined, isExecuting: false }
              : tab
          ),
        }));
      },

      // History Actions
      addToHistory: (item: Omit<QueryHistoryItem, 'id' | 'timestamp'>) => {
        const historyItem: QueryHistoryItem = {
          ...item,
          id: generateId(),
          timestamp: new Date(),
        };

        set((state) => ({
          queryHistory: [historyItem, ...state.queryHistory].slice(0, 100), // Keep last 100
        }));
      },

      clearHistory: () => {
        set({ queryHistory: [] });
      },

      // Saved Queries Actions
      setSavedQueries: (queries: SavedQuery[]) => {
        set({ savedQueries: queries });
      },

      addSavedQuery: (query: SavedQuery) => {
        set((state) => ({
          savedQueries: [...state.savedQueries, query],
        }));
      },

      updateSavedQuery: (id: string, updates: Partial<SavedQuery>) => {
        set((state) => ({
          savedQueries: state.savedQueries.map((query) =>
            query.id === id
              ? { ...query, ...updates, updatedAt: new Date() }
              : query
          ),
        }));
      },

      removeSavedQuery: (id: string) => {
        set((state) => ({
          savedQueries: state.savedQueries.filter((query) => query.id !== id),
        }));
      },

      // Explain Plan Actions
      setExplainPlan: (plan: ExplainPlan | null) => {
        set({ explainPlan: plan });
      },

      // Preferences Actions
      updatePreferences: (updates: Partial<EditorPreferences>) => {
        set((state) => ({
          preferences: { ...state.preferences, ...updates },
        }));
      },
    }),
    {
      name: 'rustydb-query-storage',
      partialize: (state) => ({
        savedQueries: state.savedQueries,
        preferences: state.preferences,
        queryHistory: state.queryHistory.slice(0, 50), // Persist only recent history
      }),
    }
  )
);
