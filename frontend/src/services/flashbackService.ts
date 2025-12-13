/**
 * Flashback Service
 * Time-travel queries and point-in-time recovery
 */

import api from './api';
import type { ApiResponse } from '../types/api';

export interface FlashbackQueryRequest {
  query: string;
  timestamp?: string;
  scn?: number;
  format?: 'json' | 'table';
}

export interface FlashbackQueryResponse {
  query_id: string;
  result: any[];
  row_count: number;
  execution_time_ms: number;
  scn_used: number;
  timestamp_used: string;
}

export interface FlashbackTableRequest {
  table_name: string;
  target_scn?: number;
  target_timestamp?: string;
  restore_triggers?: boolean;
}

export interface VersionsQueryRequest {
  table_name: string;
  start_time?: string;
  end_time?: string;
  row_filter?: string;
}

export interface RowVersion {
  row_id: string;
  scn: number;
  timestamp: string;
  operation: 'INSERT' | 'UPDATE' | 'DELETE';
  data: Record<string, any>;
  transaction_id: string;
  user: string;
}

export interface RestorePoint {
  name: string;
  scn: number;
  timestamp: string;
  guaranteed: boolean;
  description?: string;
}

export interface FlashbackDatabaseRequest {
  target_scn?: number;
  target_timestamp?: string;
  restore_point?: string;
}

export interface FlashbackStats {
  undo_retention_seconds: number;
  undo_size_mb: number;
  oldest_scn: number;
  oldest_timestamp: string;
  current_scn: number;
  flashback_enabled: boolean;
  restore_points: RestorePoint[];
}

export interface TransactionFlashbackRequest {
  transaction_id: string;
  operation: 'undo' | 'view';
}

class FlashbackService {
  /**
   * Execute flashback query
   */
  async flashbackQuery(request: FlashbackQueryRequest): Promise<ApiResponse<FlashbackQueryResponse>> {
    return api.post('/api/flashback/query', request);
  }

  /**
   * Flashback table to previous state
   */
  async flashbackTable(request: FlashbackTableRequest): Promise<ApiResponse<{ message: string; rows_affected: number }>> {
    return api.post('/api/flashback/table', request);
  }

  /**
   * Query row versions history
   */
  async queryVersions(request: VersionsQueryRequest): Promise<ApiResponse<{ versions: RowVersion[] }>> {
    return api.post('/api/flashback/versions', request);
  }

  /**
   * Create restore point
   */
  async createRestorePoint(request: { name: string; guaranteed?: boolean; description?: string }): Promise<ApiResponse<RestorePoint>> {
    return api.post('/api/flashback/restore-points', request);
  }

  /**
   * List restore points
   */
  async listRestorePoints(): Promise<ApiResponse<RestorePoint[]>> {
    return api.get('/api/flashback/restore-points');
  }

  /**
   * Delete restore point
   */
  async deleteRestorePoint(name: string): Promise<ApiResponse<{ message: string }>> {
    return api.delete(`/api/flashback/restore-points/${name}`);
  }

  /**
   * Flashback entire database
   */
  async flashbackDatabase(request: FlashbackDatabaseRequest): Promise<ApiResponse<{ message: string; scn_used: number }>> {
    return api.post('/api/flashback/database', request);
  }

  /**
   * Get flashback statistics
   */
  async getStats(): Promise<ApiResponse<FlashbackStats>> {
    return api.get('/api/flashback/stats');
  }

  /**
   * Flashback transaction
   */
  async flashbackTransaction(request: TransactionFlashbackRequest): Promise<ApiResponse<{ message: string; operations: any[] }>> {
    return api.post('/api/flashback/transaction', request);
  }

  /**
   * Get current SCN
   */
  async getCurrentScn(): Promise<ApiResponse<{ scn: number; timestamp: string }>> {
    return api.get('/api/flashback/current-scn');
  }
}

export default new FlashbackService();
