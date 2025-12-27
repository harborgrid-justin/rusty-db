import api from './api';
import { QueryResult, QueryHistoryItem, SavedQuery, ExplainPlan } from '../stores/queryStore';

export interface ExecuteQueryRequest {
  sql: string;
  params?: Record<string, unknown>;
  limit?: number;
  timeout?: number;
}

export interface ExecuteQueryResponse {
  queryId: string;
  result: QueryResult;
}

export interface PaginationParams {
  page: number;
  pageSize: number;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
}

export interface QueryHistoryResponse {
  items: QueryHistoryItem[];
  total: number;
  page: number;
  pageSize: number;
}

export interface SaveQueryRequest {
  name: string;
  sql: string;
  description?: string;
  tags?: string[];
}

class QueryService {
  /**
   * Execute a SQL query
   */
  async executeQuery(request: ExecuteQueryRequest): Promise<ExecuteQueryResponse> {
    try {
      const response = await api.post<ExecuteQueryResponse>('/api/query/execute', request);
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to execute query';
      throw new Error(message);
    }
  }

  /**
   * Get execution plan for a query
   */
  async explainQuery(sql: string): Promise<ExplainPlan> {
    try {
      const response = await api.post<ExplainPlan>('/api/query/explain', { sql });
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to get execution plan';
      throw new Error(message);
    }
  }

  /**
   * Cancel a running query
   */
  async cancelQuery(queryId: string): Promise<void> {
    try {
      await api.post(`/api/query/cancel/${queryId}`);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to cancel query';
      throw new Error(message);
    }
  }

  /**
   * Get query history with pagination
   */
  async getQueryHistory(params: PaginationParams): Promise<QueryHistoryResponse> {
    try {
      const response = await api.get<QueryHistoryResponse>('/api/query/history', {
        params,
      });
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to fetch query history';
      throw new Error(message);
    }
  }

  /**
   * Search query history
   */
  async searchQueryHistory(searchTerm: string, params: PaginationParams): Promise<QueryHistoryResponse> {
    try {
      const response = await api.get<QueryHistoryResponse>('/api/query/history/search', {
        params: { q: searchTerm, ...params },
      });
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to search query history';
      throw new Error(message);
    }
  }

  /**
   * Get all saved queries
   */
  async getSavedQueries(): Promise<SavedQuery[]> {
    try {
      const response = await api.get<SavedQuery[]>('/api/query/saved');
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to fetch saved queries';
      throw new Error(message);
    }
  }

  /**
   * Get a specific saved query
   */
  async getSavedQuery(id: string): Promise<SavedQuery> {
    try {
      const response = await api.get<SavedQuery>(`/api/query/saved/${id}`);
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to fetch saved query';
      throw new Error(message);
    }
  }

  /**
   * Save a new query
   */
  async saveQuery(request: SaveQueryRequest): Promise<SavedQuery> {
    try {
      const response = await api.post<SavedQuery>('/api/query/saved', request);
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to save query';
      throw new Error(message);
    }
  }

  /**
   * Update a saved query
   */
  async updateSavedQuery(id: string, request: Partial<SaveQueryRequest>): Promise<SavedQuery> {
    try {
      const response = await api.put<SavedQuery>(`/api/query/saved/${id}`, request);
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to update saved query';
      throw new Error(message);
    }
  }

  /**
   * Delete a saved query
   */
  async deleteQuery(id: string): Promise<void> {
    try {
      await api.delete(`/api/query/saved/${id}`);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to delete query';
      throw new Error(message);
    }
  }

  /**
   * Format SQL query
   */
  async formatQuery(sql: string): Promise<string> {
    try {
      const response = await api.post<{ formatted: string }>('/api/query/format', { sql });
      return response.data.formatted;
    } catch (error: unknown) {
      // Fallback to client-side formatting if server fails
      const message = error instanceof Error ? error.message : 'Failed to format query';
      throw new Error(message);
    }
  }

  /**
   * Validate SQL query syntax
   */
  async validateQuery(sql: string): Promise<{ valid: boolean; errors?: string[] }> {
    try {
      const response = await api.post<{ valid: boolean; errors?: string[] }>(
        '/api/query/validate',
        { sql }
      );
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to validate query';
      throw new Error(message);
    }
  }

  /**
   * Get table and column names for autocomplete
   */
  async getSchemaMetadata(): Promise<{
    tables: Array<{ name: string; schema: string; columns: Array<{ name: string; type: string }> }>;
  }> {
    try {
      const response = await api.get('/api/query/metadata');
      return response.data;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'Failed to fetch schema metadata';
      throw new Error(message);
    }
  }

  /**
   * Export query results
   */
  async exportResults(
    format: 'csv' | 'json' | 'xlsx',
    result: QueryResult
  ): Promise<Blob> {
    try {
      const response = await api.post(
        '/api/query/export',
        {
          format,
          columns: result.columns,
          rows: result.rows,
        },
        {
          responseType: 'blob',
        }
      );
      return response.data;
    } catch (error: unknown) {
      throw new Error('Failed to export results');
    }
  }

  /**
   * Client-side export to CSV
   */
  exportToCSV(result: QueryResult): string {
    const { columns, rows } = result;

    // Escape CSV values
    const escapeCsvValue = (value: string | number | boolean | null | undefined): string => {
      if (value === null || value === undefined) return '';
      const str = String(value);
      if (str.includes(',') || str.includes('"') || str.includes('\n')) {
        return `"${str.replace(/"/g, '""')}"`;
      }
      return str;
    };

    // Header row
    const header = columns.map(escapeCsvValue).join(',');

    // Data rows
    const dataRows = rows.map(row =>
      row.map(escapeCsvValue).join(',')
    );

    return [header, ...dataRows].join('\n');
  }

  /**
   * Client-side export to JSON
   */
  exportToJSON(result: QueryResult): string {
    const { columns, rows } = result;

    const data = rows.map(row => {
      const obj: Record<string, string | number | boolean | null> = {};
      columns.forEach((col, idx) => {
        obj[col] = row[idx];
      });
      return obj;
    });

    return JSON.stringify(data, null, 2);
  }

  /**
   * Download file helper
   */
  downloadFile(content: string | Blob, filename: string, mimeType: string): void {
    const blob = content instanceof Blob
      ? content
      : new Blob([content], { type: mimeType });

    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }
}

export const queryService = new QueryService();
export default queryService;
