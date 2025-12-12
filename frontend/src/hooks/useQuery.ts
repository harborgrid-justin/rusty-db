import { useState, useCallback, useEffect } from 'react';
import { useQueryStore } from '../stores/queryStore';
import queryService, { ExecuteQueryRequest, PaginationParams } from '../services/queryService';
import { QueryResult, QueryHistoryItem, SavedQuery, ExplainPlan } from '../stores/queryStore';

/**
 * Hook for executing queries
 */
export function useQueryExecution(tabId: string) {
  const { setTabExecuting, setTabResult, setTabError, addToHistory } = useQueryStore();
  const [isExecuting, setIsExecuting] = useState(false);

  const executeQuery = useCallback(
    async (sql: string, params?: Record<string, any>) => {
      if (!sql.trim()) {
        setTabError(tabId, 'Query cannot be empty');
        return;
      }

      setIsExecuting(true);
      const startTime = Date.now();

      try {
        setTabExecuting(tabId, true);

        const request: ExecuteQueryRequest = {
          sql: sql.trim(),
          params,
        };

        const response = await queryService.executeQuery(request);
        const executionTime = Date.now() - startTime;

        // Update result with execution time
        const result: QueryResult = {
          ...response.result,
          executionTime,
        };

        setTabResult(tabId, result);

        // Add to history
        addToHistory({
          sql: sql.trim(),
          executionTime,
          rowCount: result.rowCount,
        });
      } catch (error: any) {
        const errorMessage = error.message || 'Query execution failed';
        setTabError(tabId, errorMessage);

        // Add failed query to history
        addToHistory({
          sql: sql.trim(),
          executionTime: Date.now() - startTime,
          error: errorMessage,
        });
      } finally {
        setIsExecuting(false);
        setTabExecuting(tabId, false);
      }
    },
    [tabId, setTabExecuting, setTabResult, setTabError, addToHistory]
  );

  const cancelQuery = useCallback(async (queryId: string) => {
    try {
      await queryService.cancelQuery(queryId);
      setTabExecuting(tabId, false);
      setTabError(tabId, 'Query cancelled');
    } catch (error: any) {
      console.error('Failed to cancel query:', error);
    }
  }, [tabId, setTabExecuting, setTabError]);

  return {
    executeQuery,
    cancelQuery,
    isExecuting,
  };
}

/**
 * Hook for query history management
 */
export function useQueryHistory() {
  const { queryHistory, clearHistory } = useQueryStore();
  const [isLoading, setIsLoading] = useState(false);
  const [serverHistory, setServerHistory] = useState<QueryHistoryItem[]>([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [pagination, setPagination] = useState<PaginationParams>({
    page: 1,
    pageSize: 50,
  });

  const fetchHistory = useCallback(async () => {
    setIsLoading(true);
    try {
      const response = await queryService.getQueryHistory(pagination);
      setServerHistory(response.items);
    } catch (error) {
      console.error('Failed to fetch query history:', error);
    } finally {
      setIsLoading(false);
    }
  }, [pagination]);

  const searchHistory = useCallback(async (term: string) => {
    if (!term.trim()) {
      await fetchHistory();
      return;
    }

    setIsLoading(true);
    try {
      const response = await queryService.searchQueryHistory(term, pagination);
      setServerHistory(response.items);
    } catch (error) {
      console.error('Failed to search query history:', error);
    } finally {
      setIsLoading(false);
    }
  }, [pagination, fetchHistory]);

  useEffect(() => {
    if (searchTerm) {
      const debounce = setTimeout(() => searchHistory(searchTerm), 300);
      return () => clearTimeout(debounce);
    } else {
      fetchHistory();
    }
  }, [searchTerm, fetchHistory, searchHistory]);

  // Combine local and server history
  const combinedHistory = [...queryHistory, ...serverHistory].reduce((acc, item) => {
    if (!acc.find(h => h.id === item.id)) {
      acc.push(item);
    }
    return acc;
  }, [] as QueryHistoryItem[]);

  return {
    history: combinedHistory,
    isLoading,
    searchTerm,
    setSearchTerm,
    clearHistory,
    refresh: fetchHistory,
    pagination,
    setPagination,
  };
}

/**
 * Hook for saved queries management
 */
export function useSavedQueries() {
  const {
    savedQueries,
    setSavedQueries,
    addSavedQuery,
    updateSavedQuery,
    removeSavedQuery,
  } = useQueryStore();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchSavedQueries = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const queries = await queryService.getSavedQueries();
      setSavedQueries(queries);
    } catch (err: any) {
      setError(err.message);
      console.error('Failed to fetch saved queries:', err);
    } finally {
      setIsLoading(false);
    }
  }, [setSavedQueries]);

  const saveQuery = useCallback(
    async (name: string, sql: string, description?: string, tags?: string[]) => {
      setIsLoading(true);
      setError(null);
      try {
        const query = await queryService.saveQuery({ name, sql, description, tags });
        addSavedQuery(query);
        return query;
      } catch (err: any) {
        setError(err.message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [addSavedQuery]
  );

  const updateQuery = useCallback(
    async (id: string, updates: { name?: string; sql?: string; description?: string; tags?: string[] }) => {
      setIsLoading(true);
      setError(null);
      try {
        const query = await queryService.updateSavedQuery(id, updates);
        updateSavedQuery(id, query);
        return query;
      } catch (err: any) {
        setError(err.message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [updateSavedQuery]
  );

  const deleteQuery = useCallback(
    async (id: string) => {
      setIsLoading(true);
      setError(null);
      try {
        await queryService.deleteQuery(id);
        removeSavedQuery(id);
      } catch (err: any) {
        setError(err.message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [removeSavedQuery]
  );

  useEffect(() => {
    fetchSavedQueries();
  }, [fetchSavedQueries]);

  return {
    savedQueries,
    isLoading,
    error,
    saveQuery,
    updateQuery,
    deleteQuery,
    refresh: fetchSavedQueries,
  };
}

/**
 * Hook for explain plan visualization
 */
export function useExplainPlan() {
  const { explainPlan, setExplainPlan } = useQueryStore();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const explain = useCallback(
    async (sql: string) => {
      if (!sql.trim()) {
        setError('Query cannot be empty');
        return;
      }

      setIsLoading(true);
      setError(null);
      try {
        const plan = await queryService.explainQuery(sql.trim());
        setExplainPlan(plan);
        return plan;
      } catch (err: any) {
        setError(err.message);
        setExplainPlan(null);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [setExplainPlan]
  );

  const clearPlan = useCallback(() => {
    setExplainPlan(null);
    setError(null);
  }, [setExplainPlan]);

  return {
    explainPlan,
    isLoading,
    error,
    explain,
    clearPlan,
  };
}

/**
 * Hook for SQL formatting
 */
export function useSqlFormatter() {
  const [isFormatting, setIsFormatting] = useState(false);

  const formatSql = useCallback(async (sql: string): Promise<string> => {
    setIsFormatting(true);
    try {
      // Try server-side formatting first
      try {
        const formatted = await queryService.formatQuery(sql);
        return formatted;
      } catch {
        // Fallback to client-side formatting using sql-formatter
        const { format } = await import('sql-formatter');
        return format(sql, {
          language: 'sql',
          uppercase: true,
          linesBetweenQueries: 2,
        });
      }
    } finally {
      setIsFormatting(false);
    }
  }, []);

  return {
    formatSql,
    isFormatting,
  };
}

/**
 * Hook for schema metadata (autocomplete)
 */
export function useSchemaMetadata() {
  const [metadata, setMetadata] = useState<{
    tables: Array<{ name: string; schema: string; columns: Array<{ name: string; type: string }> }>;
  } | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const fetchMetadata = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await queryService.getSchemaMetadata();
      setMetadata(data);
    } catch (error) {
      console.error('Failed to fetch schema metadata:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchMetadata();
  }, [fetchMetadata]);

  return {
    metadata,
    isLoading,
    refresh: fetchMetadata,
  };
}

/**
 * Hook for exporting query results
 */
export function useExportResults() {
  const [isExporting, setIsExporting] = useState(false);

  const exportResults = useCallback(
    async (format: 'csv' | 'json' | 'xlsx', result: QueryResult, filename?: string) => {
      setIsExporting(true);
      try {
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const defaultFilename = `query-results-${timestamp}`;

        if (format === 'csv') {
          const csv = queryService.exportToCSV(result);
          queryService.downloadFile(
            csv,
            filename || `${defaultFilename}.csv`,
            'text/csv'
          );
        } else if (format === 'json') {
          const json = queryService.exportToJSON(result);
          queryService.downloadFile(
            json,
            filename || `${defaultFilename}.json`,
            'application/json'
          );
        } else if (format === 'xlsx') {
          // Try server-side export for Excel
          const blob = await queryService.exportResults(format, result);
          queryService.downloadFile(
            blob,
            filename || `${defaultFilename}.xlsx`,
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
          );
        }
      } catch (error) {
        console.error('Failed to export results:', error);
        throw error;
      } finally {
        setIsExporting(false);
      }
    },
    []
  );

  return {
    exportResults,
    isExporting,
  };
}
