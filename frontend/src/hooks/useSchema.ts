// ============================================================================
// Schema Data Hooks
// Custom React hooks for schema operations
// ============================================================================

import { useState, useEffect, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import type {
  Table,
  Index,
  View,
  Column,
  ForeignKey,
  Constraint,
  PaginationParams,
} from '../types';
import * as schemaService from '../services/schemaService';
import { getErrorMessage } from '../services/api';

// ============================================================================
// Query Keys
// ============================================================================

export const schemaKeys = {
  all: ['schema'] as const,
  tables: () => [...schemaKeys.all, 'tables'] as const,
  table: (name: string, schema?: string) =>
    [...schemaKeys.tables(), name, schema] as const,
  tableStats: (name: string, schema?: string) =>
    [...schemaKeys.table(name, schema), 'stats'] as const,
  tableDDL: (name: string, schema?: string) =>
    [...schemaKeys.table(name, schema), 'ddl'] as const,
  columns: (tableName: string, schema?: string) =>
    [...schemaKeys.table(tableName, schema), 'columns'] as const,
  indexes: (tableName?: string) => [...schemaKeys.all, 'indexes', tableName] as const,
  index: (name: string, schema?: string) =>
    [...schemaKeys.indexes(), name, schema] as const,
  views: () => [...schemaKeys.all, 'views'] as const,
  view: (name: string, schema?: string) =>
    [...schemaKeys.views(), name, schema] as const,
  procedures: () => [...schemaKeys.all, 'procedures'] as const,
  procedure: (name: string, schema?: string) =>
    [...schemaKeys.procedures(), name, schema] as const,
  foreignKeys: (tableName: string, schema?: string) =>
    [...schemaKeys.table(tableName, schema), 'foreignKeys'] as const,
  constraints: (tableName: string, schema?: string) =>
    [...schemaKeys.table(tableName, schema), 'constraints'] as const,
};

// ============================================================================
// Table Hooks
// ============================================================================

/**
 * Hook to fetch tables list with pagination and filtering
 */
export function useTables(
  params?: Partial<PaginationParams> & {
    schema?: string;
    search?: string;
    includeSystem?: boolean;
  }
) {
  return useQuery({
    queryKey: [...schemaKeys.tables(), params],
    queryFn: async () => {
      const response = await schemaService.getTables(params);
      return response.data!;
    },
    staleTime: 30000, // 30 seconds
  });
}

/**
 * Hook to fetch a single table's details
 */
export function useTable(tableName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.table(tableName, schema),
    queryFn: async () => {
      const response = await schemaService.getTable(tableName, schema);
      return response.data!;
    },
    enabled: !!tableName,
  });
}

/**
 * Hook to fetch table statistics
 */
export function useTableStats(tableName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.tableStats(tableName, schema),
    queryFn: async () => {
      const response = await schemaService.getTableStats(tableName, schema);
      return response.data!;
    },
    enabled: !!tableName,
    refetchInterval: 10000, // Refresh every 10 seconds
  });
}

/**
 * Hook to fetch table DDL
 */
export function useTableDDL(tableName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.tableDDL(tableName, schema),
    queryFn: async () => {
      const response = await schemaService.getTableDDL(tableName, schema);
      return response.data!;
    },
    enabled: !!tableName,
  });
}

/**
 * Hook to create a table
 */
export function useCreateTable() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (definition: schemaService.CreateTableRequest) =>
      schemaService.createTable(definition),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.tables() });
    },
  });
}

/**
 * Hook to alter a table
 */
export function useAlterTable(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (changes: schemaService.AlterTableRequest) =>
      schemaService.alterTable(tableName, changes, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
      queryClient.invalidateQueries({ queryKey: schemaKeys.tables() });
    },
  });
}

/**
 * Hook to drop a table
 */
export function useDropTable() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      tableName,
      schema = 'public',
      cascade = false,
    }: {
      tableName: string;
      schema?: string;
      cascade?: boolean;
    }) => schemaService.dropTable(tableName, schema, cascade),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.tables() });
    },
  });
}

/**
 * Hook to truncate a table
 */
export function useTruncateTable() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      tableName,
      schema = 'public',
      cascade = false,
    }: {
      tableName: string;
      schema?: string;
      cascade?: boolean;
    }) => schemaService.truncateTable(tableName, schema, cascade),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: schemaKeys.tableStats(variables.tableName, variables.schema),
      });
    },
  });
}

// ============================================================================
// Column Hooks
// ============================================================================

/**
 * Hook to fetch columns for a table
 */
export function useColumns(tableName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.columns(tableName, schema),
    queryFn: async () => {
      const response = await schemaService.getColumns(tableName, schema);
      return response.data!;
    },
    enabled: !!tableName,
  });
}

/**
 * Hook to add a column
 */
export function useAddColumn(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (column: schemaService.ColumnDefinition) =>
      schemaService.addColumn(tableName, column, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.columns(tableName, schema) });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

/**
 * Hook to modify a column
 */
export function useModifyColumn(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      columnName,
      changes,
    }: {
      columnName: string;
      changes: Partial<schemaService.ColumnDefinition>;
    }) => schemaService.modifyColumn(tableName, columnName, changes, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.columns(tableName, schema) });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

/**
 * Hook to drop a column
 */
export function useDropColumn(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      columnName,
      cascade = false,
    }: {
      columnName: string;
      cascade?: boolean;
    }) => schemaService.dropColumn(tableName, columnName, schema, cascade),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.columns(tableName, schema) });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

// ============================================================================
// Index Hooks
// ============================================================================

/**
 * Hook to fetch indexes
 */
export function useIndexes(params?: {
  tableName?: string;
  schema?: string;
  includeUnused?: boolean;
}) {
  return useQuery({
    queryKey: [...schemaKeys.indexes(params?.tableName), params],
    queryFn: async () => {
      const response = await schemaService.getIndexes(params);
      return response.data!;
    },
  });
}

/**
 * Hook to fetch a single index
 */
export function useIndex(indexName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.index(indexName, schema),
    queryFn: async () => {
      const response = await schemaService.getIndex(indexName, schema);
      return response.data!;
    },
    enabled: !!indexName,
  });
}

/**
 * Hook to create an index
 */
export function useCreateIndex() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (definition: schemaService.CreateIndexRequest) =>
      schemaService.createIndex(definition),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.indexes(variables.tableName) });
      if (variables.tableName) {
        queryClient.invalidateQueries({
          queryKey: schemaKeys.table(variables.tableName, variables.schema),
        });
      }
    },
  });
}

/**
 * Hook to drop an index
 */
export function useDropIndex() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      indexName,
      schema = 'public',
      concurrent = false,
    }: {
      indexName: string;
      schema?: string;
      concurrent?: boolean;
    }) => schemaService.dropIndex(indexName, schema, concurrent),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.indexes() });
    },
  });
}

/**
 * Hook to reindex
 */
export function useReindex() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      target,
      schema = 'public',
      concurrent = false,
    }: {
      target: string;
      schema?: string;
      concurrent?: boolean;
    }) => schemaService.reindex(target, schema, concurrent),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.indexes() });
    },
  });
}

/**
 * Hook to fetch unused indexes
 */
export function useUnusedIndexes(minSizeMB: number = 1) {
  return useQuery({
    queryKey: [...schemaKeys.indexes(), 'unused', minSizeMB],
    queryFn: async () => {
      const response = await schemaService.getUnusedIndexes(minSizeMB);
      return response.data!;
    },
  });
}

/**
 * Hook to fetch index recommendations
 */
export function useIndexRecommendations() {
  return useQuery({
    queryKey: [...schemaKeys.indexes(), 'recommendations'],
    queryFn: async () => {
      const response = await schemaService.getIndexRecommendations();
      return response.data!;
    },
  });
}

// ============================================================================
// View Hooks
// ============================================================================

/**
 * Hook to fetch views
 */
export function useViews(params?: {
  schema?: string;
  materializedOnly?: boolean;
  search?: string;
}) {
  return useQuery({
    queryKey: [...schemaKeys.views(), params],
    queryFn: async () => {
      const response = await schemaService.getViews(params);
      return response.data!;
    },
  });
}

/**
 * Hook to fetch a single view
 */
export function useView(viewName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.view(viewName, schema),
    queryFn: async () => {
      const response = await schemaService.getView(viewName, schema);
      return response.data!;
    },
    enabled: !!viewName,
  });
}

/**
 * Hook to create a view
 */
export function useCreateView() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (definition: schemaService.CreateViewRequest) =>
      schemaService.createView(definition),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.views() });
    },
  });
}

/**
 * Hook to drop a view
 */
export function useDropView() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      viewName,
      schema = 'public',
      cascade = false,
      materialized = false,
    }: {
      viewName: string;
      schema?: string;
      cascade?: boolean;
      materialized?: boolean;
    }) => schemaService.dropView(viewName, schema, cascade, materialized),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.views() });
    },
  });
}

/**
 * Hook to refresh a materialized view
 */
export function useRefreshMaterializedView() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      viewName,
      options = {},
      schema = 'public',
    }: {
      viewName: string;
      options?: schemaService.RefreshMaterializedViewRequest;
      schema?: string;
    }) => schemaService.refreshMaterializedView(viewName, options, schema),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: schemaKeys.view(variables.viewName, variables.schema),
      });
    },
  });
}

/**
 * Hook to fetch view dependencies
 */
export function useViewDependencies(viewName: string, schema: string = 'public') {
  return useQuery({
    queryKey: [...schemaKeys.view(viewName, schema), 'dependencies'],
    queryFn: async () => {
      const response = await schemaService.getViewDependencies(viewName, schema);
      return response.data!;
    },
    enabled: !!viewName,
  });
}

// ============================================================================
// Stored Procedure Hooks
// ============================================================================

/**
 * Hook to fetch stored procedures
 */
export function useProcedures(params?: { schema?: string; search?: string }) {
  return useQuery({
    queryKey: [...schemaKeys.procedures(), params],
    queryFn: async () => {
      const response = await schemaService.getProcedures(params);
      return response.data!;
    },
  });
}

/**
 * Hook to fetch a single stored procedure
 */
export function useProcedure(procedureName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.procedure(procedureName, schema),
    queryFn: async () => {
      const response = await schemaService.getProcedure(procedureName, schema);
      return response.data!;
    },
    enabled: !!procedureName,
  });
}

/**
 * Hook to create a stored procedure
 */
export function useCreateProcedure() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (definition: schemaService.CreateProcedureRequest) =>
      schemaService.createProcedure(definition),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.procedures() });
    },
  });
}

/**
 * Hook to drop a stored procedure
 */
export function useDropProcedure() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      procedureName,
      schema = 'public',
      cascade = false,
    }: {
      procedureName: string;
      schema?: string;
      cascade?: boolean;
    }) => schemaService.dropProcedure(procedureName, schema, cascade),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.procedures() });
    },
  });
}

/**
 * Hook to execute a stored procedure
 */
export function useExecuteProcedure(procedureName: string, schema: string = 'public') {
  return useMutation({
    mutationFn: (request: schemaService.ExecuteProcedureRequest) =>
      schemaService.executeProcedure(procedureName, request, schema),
  });
}

// ============================================================================
// Data Browsing Hooks
// ============================================================================

/**
 * Hook to browse table data
 */
export function useBrowseTableData(
  tableName: string,
  params: schemaService.BrowseDataRequest,
  schema: string = 'public'
) {
  return useQuery({
    queryKey: [...schemaKeys.table(tableName, schema), 'data', params],
    queryFn: async () => {
      const response = await schemaService.browseTableData(tableName, params, schema);
      return response.data!;
    },
    enabled: !!tableName,
    keepPreviousData: true,
  });
}

/**
 * Hook to update a row
 */
export function useUpdateRow(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: schemaService.UpdateRowRequest) =>
      schemaService.updateRow(tableName, request, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

/**
 * Hook to insert a row
 */
export function useInsertRow(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: schemaService.InsertRowRequest) =>
      schemaService.insertRow(tableName, request, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

/**
 * Hook to delete a row
 */
export function useDeleteRow(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (primaryKey: Record<string, unknown>) =>
      schemaService.deleteRow(tableName, primaryKey, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

// ============================================================================
// Foreign Key Hooks
// ============================================================================

/**
 * Hook to fetch foreign keys for a table
 */
export function useForeignKeys(tableName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.foreignKeys(tableName, schema),
    queryFn: async () => {
      const response = await schemaService.getForeignKeys(tableName, schema);
      return response.data!;
    },
    enabled: !!tableName,
  });
}

/**
 * Hook to add a foreign key
 */
export function useAddForeignKey(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (foreignKey: schemaService.ForeignKeyDefinition) =>
      schemaService.addForeignKey(tableName, foreignKey, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: schemaKeys.foreignKeys(tableName, schema),
      });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

/**
 * Hook to drop a foreign key
 */
export function useDropForeignKey(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (constraintName: string) =>
      schemaService.dropForeignKey(tableName, constraintName, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: schemaKeys.foreignKeys(tableName, schema),
      });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

// ============================================================================
// Constraint Hooks
// ============================================================================

/**
 * Hook to fetch constraints for a table
 */
export function useConstraints(tableName: string, schema: string = 'public') {
  return useQuery({
    queryKey: schemaKeys.constraints(tableName, schema),
    queryFn: async () => {
      const response = await schemaService.getConstraints(tableName, schema);
      return response.data!;
    },
    enabled: !!tableName,
  });
}

/**
 * Hook to add a constraint
 */
export function useAddConstraint(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (constraint: schemaService.ConstraintDefinition) =>
      schemaService.addConstraint(tableName, constraint, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: schemaKeys.constraints(tableName, schema),
      });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}

/**
 * Hook to drop a constraint
 */
export function useDropConstraint(tableName: string, schema: string = 'public') {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (constraintName: string) =>
      schemaService.dropConstraint(tableName, constraintName, schema),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: schemaKeys.constraints(tableName, schema),
      });
      queryClient.invalidateQueries({ queryKey: schemaKeys.table(tableName, schema) });
    },
  });
}
