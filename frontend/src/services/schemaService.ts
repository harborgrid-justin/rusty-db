// ============================================================================
// RustyDB Schema Service
// Handles all schema-related API operations
// ============================================================================

import { get, post, put, del } from './api';
import type {
  ApiResponse,
  Table,
  Column,
  Index,
  View,
  MaterializedView,
  ForeignKey,
  Constraint,
  PaginatedResponse,
  PaginationParams,
} from '../types';

// ============================================================================
// Table Operations
// ============================================================================

export interface CreateTableRequest {
  name: string;
  schema?: string;
  columns: ColumnDefinition[];
  primaryKey?: string[];
  foreignKeys?: ForeignKeyDefinition[];
  constraints?: ConstraintDefinition[];
  comment?: string;
}

export interface ColumnDefinition {
  name: string;
  dataType: string;
  nullable?: boolean;
  defaultValue?: string;
  comment?: string;
  length?: number;
  precision?: number;
  scale?: number;
}

export interface ForeignKeyDefinition {
  name?: string;
  columns: string[];
  referencedTable: string;
  referencedColumns: string[];
  onDelete?: 'cascade' | 'restrict' | 'set_null' | 'set_default' | 'no_action';
  onUpdate?: 'cascade' | 'restrict' | 'set_null' | 'set_default' | 'no_action';
}

export interface ConstraintDefinition {
  name?: string;
  type: 'unique' | 'check' | 'not_null' | 'exclusion';
  columns: string[];
  expression?: string;
  isDeferred?: boolean;
}

export interface AlterTableRequest {
  addColumns?: ColumnDefinition[];
  dropColumns?: string[];
  modifyColumns?: {
    name: string;
    changes: Partial<ColumnDefinition>;
  }[];
  renameColumns?: {
    oldName: string;
    newName: string;
  }[];
  addConstraints?: ConstraintDefinition[];
  dropConstraints?: string[];
  renameTo?: string;
}

export interface TableStats {
  rowCount: number;
  size: number;
  indexSize: number;
  totalSize: number;
  lastVacuum?: string;
  lastAnalyze?: string;
  lastAutoVacuum?: string;
  lastAutoAnalyze?: string;
}

/**
 * Get all tables with optional filtering and pagination
 */
export async function getTables(
  params?: Partial<PaginationParams> & {
    schema?: string;
    search?: string;
    includeSystem?: boolean;
  }
): Promise<ApiResponse<PaginatedResponse<Table>>> {
  const queryParams = new URLSearchParams();

  if (params?.page) queryParams.set('page', params.page.toString());
  if (params?.pageSize) queryParams.set('pageSize', params.pageSize.toString());
  if (params?.sortBy) queryParams.set('sortBy', params.sortBy);
  if (params?.sortOrder) queryParams.set('sortOrder', params.sortOrder);
  if (params?.schema) queryParams.set('schema', params.schema);
  if (params?.search) queryParams.set('search', params.search);
  if (params?.includeSystem !== undefined) {
    queryParams.set('includeSystem', params.includeSystem.toString());
  }

  const query = queryParams.toString();
  return get<PaginatedResponse<Table>>(`/schema/tables${query ? `?${query}` : ''}`);
}

/**
 * Get details for a specific table
 */
export async function getTable(
  tableName: string,
  schema: string = 'public'
): Promise<ApiResponse<Table>> {
  return get<Table>(`/schema/tables/${schema}.${tableName}`);
}

/**
 * Create a new table
 */
export async function createTable(
  definition: CreateTableRequest
): Promise<ApiResponse<Table>> {
  return post<Table>('/schema/tables', definition);
}

/**
 * Alter an existing table
 */
export async function alterTable(
  tableName: string,
  changes: AlterTableRequest,
  schema: string = 'public'
): Promise<ApiResponse<Table>> {
  return put<Table>(`/schema/tables/${schema}.${tableName}`, changes);
}

/**
 * Drop a table
 */
export async function dropTable(
  tableName: string,
  schema: string = 'public',
  cascade: boolean = false
): Promise<ApiResponse<void>> {
  return del<void>(`/schema/tables/${schema}.${tableName}?cascade=${cascade}`);
}

/**
 * Truncate a table
 */
export async function truncateTable(
  tableName: string,
  schema: string = 'public',
  cascade: boolean = false
): Promise<ApiResponse<void>> {
  return post<void>(`/schema/tables/${schema}.${tableName}/truncate`, { cascade });
}

/**
 * Get table statistics
 */
export async function getTableStats(
  tableName: string,
  schema: string = 'public'
): Promise<ApiResponse<TableStats>> {
  return get<TableStats>(`/schema/tables/${schema}.${tableName}/stats`);
}

/**
 * Get table DDL
 */
export async function getTableDDL(
  tableName: string,
  schema: string = 'public'
): Promise<ApiResponse<{ ddl: string }>> {
  return get<{ ddl: string }>(`/schema/tables/${schema}.${tableName}/ddl`);
}

// ============================================================================
// Column Operations
// ============================================================================

/**
 * Get columns for a table
 */
export async function getColumns(
  tableName: string,
  schema: string = 'public'
): Promise<ApiResponse<Column[]>> {
  return get<Column[]>(`/schema/tables/${schema}.${tableName}/columns`);
}

/**
 * Add a column to a table
 */
export async function addColumn(
  tableName: string,
  column: ColumnDefinition,
  schema: string = 'public'
): Promise<ApiResponse<Column>> {
  return post<Column>(`/schema/tables/${schema}.${tableName}/columns`, column);
}

/**
 * Modify a column
 */
export async function modifyColumn(
  tableName: string,
  columnName: string,
  changes: Partial<ColumnDefinition>,
  schema: string = 'public'
): Promise<ApiResponse<Column>> {
  return put<Column>(
    `/schema/tables/${schema}.${tableName}/columns/${columnName}`,
    changes
  );
}

/**
 * Drop a column
 */
export async function dropColumn(
  tableName: string,
  columnName: string,
  schema: string = 'public',
  cascade: boolean = false
): Promise<ApiResponse<void>> {
  return del<void>(
    `/schema/tables/${schema}.${tableName}/columns/${columnName}?cascade=${cascade}`
  );
}

// ============================================================================
// Index Operations
// ============================================================================

export interface CreateIndexRequest {
  name?: string;
  tableName: string;
  schema?: string;
  columns: string[];
  type?: 'btree' | 'hash' | 'gist' | 'gin' | 'brin' | 'spgist';
  unique?: boolean;
  concurrent?: boolean;
  where?: string;
  include?: string[];
}

export interface IndexRecommendation {
  tableName: string;
  columns: string[];
  reason: string;
  estimatedImprovement: number;
  queryPatterns: string[];
}

/**
 * Get all indexes with optional filtering
 */
export async function getIndexes(
  params?: {
    tableName?: string;
    schema?: string;
    includeUnused?: boolean;
  }
): Promise<ApiResponse<Index[]>> {
  const queryParams = new URLSearchParams();

  if (params?.tableName) queryParams.set('tableName', params.tableName);
  if (params?.schema) queryParams.set('schema', params.schema);
  if (params?.includeUnused !== undefined) {
    queryParams.set('includeUnused', params.includeUnused.toString());
  }

  const query = queryParams.toString();
  return get<Index[]>(`/schema/indexes${query ? `?${query}` : ''}`);
}

/**
 * Get a specific index
 */
export async function getIndex(
  indexName: string,
  schema: string = 'public'
): Promise<ApiResponse<Index>> {
  return get<Index>(`/schema/indexes/${schema}.${indexName}`);
}

/**
 * Create a new index
 */
export async function createIndex(
  definition: CreateIndexRequest
): Promise<ApiResponse<Index>> {
  return post<Index>('/schema/indexes', definition);
}

/**
 * Drop an index
 */
export async function dropIndex(
  indexName: string,
  schema: string = 'public',
  concurrent: boolean = false
): Promise<ApiResponse<void>> {
  return del<void>(`/schema/indexes/${schema}.${indexName}?concurrent=${concurrent}`);
}

/**
 * Reindex a table or index
 */
export async function reindex(
  target: string,
  schema: string = 'public',
  concurrent: boolean = false
): Promise<ApiResponse<void>> {
  return post<void>(`/schema/indexes/${schema}.${target}/reindex`, { concurrent });
}

/**
 * Get unused indexes
 */
export async function getUnusedIndexes(
  minSizeMB: number = 1
): Promise<ApiResponse<Index[]>> {
  return get<Index[]>(`/schema/indexes/unused?minSize=${minSizeMB * 1024 * 1024}`);
}

/**
 * Get index recommendations
 */
export async function getIndexRecommendations(): Promise<
  ApiResponse<IndexRecommendation[]>
> {
  return get<IndexRecommendation[]>('/schema/indexes/recommendations');
}

// ============================================================================
// View Operations
// ============================================================================

export interface CreateViewRequest {
  name: string;
  schema?: string;
  definition: string;
  materialized?: boolean;
  replace?: boolean;
  comment?: string;
}

export interface RefreshMaterializedViewRequest {
  concurrent?: boolean;
  withData?: boolean;
}

/**
 * Get all views
 */
export async function getViews(
  params?: {
    schema?: string;
    materializedOnly?: boolean;
    search?: string;
  }
): Promise<ApiResponse<View[]>> {
  const queryParams = new URLSearchParams();

  if (params?.schema) queryParams.set('schema', params.schema);
  if (params?.materializedOnly !== undefined) {
    queryParams.set('materializedOnly', params.materializedOnly.toString());
  }
  if (params?.search) queryParams.set('search', params.search);

  const query = queryParams.toString();
  return get<View[]>(`/schema/views${query ? `?${query}` : ''}`);
}

/**
 * Get a specific view
 */
export async function getView(
  viewName: string,
  schema: string = 'public'
): Promise<ApiResponse<View>> {
  return get<View>(`/schema/views/${schema}.${viewName}`);
}

/**
 * Create a new view
 */
export async function createView(
  definition: CreateViewRequest
): Promise<ApiResponse<View>> {
  return post<View>('/schema/views', definition);
}

/**
 * Drop a view
 */
export async function dropView(
  viewName: string,
  schema: string = 'public',
  cascade: boolean = false,
  materialized: boolean = false
): Promise<ApiResponse<void>> {
  return del<void>(
    `/schema/views/${schema}.${viewName}?cascade=${cascade}&materialized=${materialized}`
  );
}

/**
 * Refresh a materialized view
 */
export async function refreshMaterializedView(
  viewName: string,
  options: RefreshMaterializedViewRequest = {},
  schema: string = 'public'
): Promise<ApiResponse<void>> {
  return post<void>(`/schema/views/${schema}.${viewName}/refresh`, options);
}

/**
 * Get view dependencies
 */
export async function getViewDependencies(
  viewName: string,
  schema: string = 'public'
): Promise<ApiResponse<{ dependencies: string[]; dependents: string[] }>> {
  return get<{ dependencies: string[]; dependents: string[] }>(
    `/schema/views/${schema}.${viewName}/dependencies`
  );
}

// ============================================================================
// Stored Procedure Operations
// ============================================================================

export interface StoredProcedure {
  name: string;
  schema: string;
  language: string;
  returnType: string;
  parameters: ProcedureParameter[];
  definition: string;
  isStrict: boolean;
  volatility: 'volatile' | 'stable' | 'immutable';
  securityDefiner: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ProcedureParameter {
  name: string;
  type: string;
  mode: 'in' | 'out' | 'inout' | 'variadic';
  defaultValue?: string;
}

export interface CreateProcedureRequest {
  name: string;
  schema?: string;
  language: string;
  returnType: string;
  parameters?: ProcedureParameter[];
  definition: string;
  replace?: boolean;
  strict?: boolean;
  volatility?: 'volatile' | 'stable' | 'immutable';
  securityDefiner?: boolean;
}

export interface ExecuteProcedureRequest {
  parameters: Record<string, unknown>;
}

/**
 * Get all stored procedures
 */
export async function getProcedures(
  params?: {
    schema?: string;
    search?: string;
  }
): Promise<ApiResponse<StoredProcedure[]>> {
  const queryParams = new URLSearchParams();

  if (params?.schema) queryParams.set('schema', params.schema);
  if (params?.search) queryParams.set('search', params.search);

  const query = queryParams.toString();
  return get<StoredProcedure[]>(`/schema/procedures${query ? `?${query}` : ''}`);
}

/**
 * Get a specific stored procedure
 */
export async function getProcedure(
  procedureName: string,
  schema: string = 'public'
): Promise<ApiResponse<StoredProcedure>> {
  return get<StoredProcedure>(`/schema/procedures/${schema}.${procedureName}`);
}

/**
 * Create a new stored procedure
 */
export async function createProcedure(
  definition: CreateProcedureRequest
): Promise<ApiResponse<StoredProcedure>> {
  return post<StoredProcedure>('/schema/procedures', definition);
}

/**
 * Drop a stored procedure
 */
export async function dropProcedure(
  procedureName: string,
  schema: string = 'public',
  cascade: boolean = false
): Promise<ApiResponse<void>> {
  return del<void>(`/schema/procedures/${schema}.${procedureName}?cascade=${cascade}`);
}

/**
 * Execute a stored procedure
 */
export async function executeProcedure(
  procedureName: string,
  request: ExecuteProcedureRequest,
  schema: string = 'public'
): Promise<ApiResponse<unknown>> {
  return post<unknown>(`/schema/procedures/${schema}.${procedureName}/execute`, request);
}

// ============================================================================
// Data Browsing Operations
// ============================================================================

export interface BrowseDataRequest extends PaginationParams {
  filter?: Record<string, unknown>;
  orderBy?: string[];
}

export interface BrowseDataResponse {
  columns: Column[];
  rows: Record<string, unknown>[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface UpdateRowRequest {
  primaryKey: Record<string, unknown>;
  values: Record<string, unknown>;
}

export interface InsertRowRequest {
  values: Record<string, unknown>;
}

/**
 * Browse table data with pagination and filtering
 */
export async function browseTableData(
  tableName: string,
  params: BrowseDataRequest,
  schema: string = 'public'
): Promise<ApiResponse<BrowseDataResponse>> {
  return post<BrowseDataResponse>(
    `/schema/tables/${schema}.${tableName}/browse`,
    params
  );
}

/**
 * Update a row in a table
 */
export async function updateRow(
  tableName: string,
  request: UpdateRowRequest,
  schema: string = 'public'
): Promise<ApiResponse<Record<string, unknown>>> {
  return put<Record<string, unknown>>(
    `/schema/tables/${schema}.${tableName}/rows`,
    request
  );
}

/**
 * Insert a row into a table
 */
export async function insertRow(
  tableName: string,
  request: InsertRowRequest,
  schema: string = 'public'
): Promise<ApiResponse<Record<string, unknown>>> {
  return post<Record<string, unknown>>(
    `/schema/tables/${schema}.${tableName}/rows`,
    request
  );
}

/**
 * Delete a row from a table
 */
export async function deleteRow(
  tableName: string,
  primaryKey: Record<string, unknown>,
  schema: string = 'public'
): Promise<ApiResponse<void>> {
  return del<void>(`/schema/tables/${schema}.${tableName}/rows`, {
    data: { primaryKey },
  });
}

// ============================================================================
// Schema Export/Import Operations
// ============================================================================

export interface ExportSchemaRequest {
  tables?: string[];
  includeData?: boolean;
  format?: 'sql' | 'json';
}

export interface ImportSchemaRequest {
  data: string;
  format: 'sql' | 'json';
  dropExisting?: boolean;
}

/**
 * Export schema definition
 */
export async function exportSchema(
  request: ExportSchemaRequest,
  schema: string = 'public'
): Promise<ApiResponse<{ content: string; format: string }>> {
  return post<{ content: string; format: string }>(
    `/schema/${schema}/export`,
    request
  );
}

/**
 * Import schema definition
 */
export async function importSchema(
  request: ImportSchemaRequest,
  schema: string = 'public'
): Promise<ApiResponse<{ tablesCreated: number; rowsImported?: number }>> {
  return post<{ tablesCreated: number; rowsImported?: number }>(
    `/schema/${schema}/import`,
    request
  );
}

// ============================================================================
// Foreign Key Operations
// ============================================================================

/**
 * Get foreign keys for a table
 */
export async function getForeignKeys(
  tableName: string,
  schema: string = 'public'
): Promise<ApiResponse<ForeignKey[]>> {
  return get<ForeignKey[]>(`/schema/tables/${schema}.${tableName}/foreign-keys`);
}

/**
 * Add a foreign key constraint
 */
export async function addForeignKey(
  tableName: string,
  foreignKey: ForeignKeyDefinition,
  schema: string = 'public'
): Promise<ApiResponse<ForeignKey>> {
  return post<ForeignKey>(
    `/schema/tables/${schema}.${tableName}/foreign-keys`,
    foreignKey
  );
}

/**
 * Drop a foreign key constraint
 */
export async function dropForeignKey(
  tableName: string,
  constraintName: string,
  schema: string = 'public'
): Promise<ApiResponse<void>> {
  return del<void>(
    `/schema/tables/${schema}.${tableName}/foreign-keys/${constraintName}`
  );
}

// ============================================================================
// Constraint Operations
// ============================================================================

/**
 * Get constraints for a table
 */
export async function getConstraints(
  tableName: string,
  schema: string = 'public'
): Promise<ApiResponse<Constraint[]>> {
  return get<Constraint[]>(`/schema/tables/${schema}.${tableName}/constraints`);
}

/**
 * Add a constraint
 */
export async function addConstraint(
  tableName: string,
  constraint: ConstraintDefinition,
  schema: string = 'public'
): Promise<ApiResponse<Constraint>> {
  return post<Constraint>(
    `/schema/tables/${schema}.${tableName}/constraints`,
    constraint
  );
}

/**
 * Drop a constraint
 */
export async function dropConstraint(
  tableName: string,
  constraintName: string,
  schema: string = 'public'
): Promise<ApiResponse<void>> {
  return del<void>(
    `/schema/tables/${schema}.${tableName}/constraints/${constraintName}`
  );
}
