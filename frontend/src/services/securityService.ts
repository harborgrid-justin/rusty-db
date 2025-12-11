// ============================================================================
// Security Service - API calls for security management
// ============================================================================

import { get, post, put, del, buildQueryParams } from './api';
import type {
  ApiResponse,
  PaginatedResponse,
  EncryptionKey,
  DataMaskingPolicy,
  AuditLog,
  AuditEventType,
  EncryptionAlgorithm,
  KeyType,
  MaskingType,
} from '../types';

// ============================================================================
// Security Overview Types
// ============================================================================

export interface SecurityOverview {
  encryptionStatus: {
    enabled: boolean;
    activeKeys: number;
    encryptedTables: number;
    expiringKeys: number;
  };
  maskingStatus: {
    activePolicies: number;
    maskedColumns: number;
    affectedTables: number;
  };
  auditStatus: {
    eventsToday: number;
    failedAuthentications: number;
    suspiciousActivities: number;
  };
  complianceScore: number;
  lastSecurityScan?: string;
}

export interface SecurityAlert {
  id: string;
  timestamp: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  type: 'encryption' | 'authentication' | 'authorization' | 'data_access' | 'configuration';
  title: string;
  description: string;
  resolved: boolean;
}

// ============================================================================
// Encryption Management
// ============================================================================

export interface CreateEncryptionKeyRequest {
  name: string;
  algorithm: EncryptionAlgorithm;
  keyType: KeyType;
  expiresInDays?: number;
  metadata?: Record<string, unknown>;
}

export interface RotateKeyRequest {
  gracePeriodHours?: number;
  notifyUsers?: boolean;
}

export interface KeyRotationStatus {
  keyId: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  progress: number;
  startTime: string;
  estimatedCompletion?: string;
  affectedTables: string[];
  errors?: string[];
}

export interface EncryptedTable {
  schema: string;
  table: string;
  encryptedColumns: string[];
  keyId: string;
  encryptedAt: string;
}

// ============================================================================
// Data Masking
// ============================================================================

export interface CreateMaskingPolicyRequest {
  name: string;
  description?: string;
  table: string;
  column: string;
  maskingType: MaskingType;
  maskingFunction?: string;
  applyTo: string[];
}

export interface UpdateMaskingPolicyRequest {
  name?: string;
  description?: string;
  maskingType?: MaskingType;
  maskingFunction?: string;
  applyTo?: string[];
  isEnabled?: boolean;
}

export interface TestMaskingRequest {
  maskingType: MaskingType;
  maskingFunction?: string;
  sampleData: unknown[];
}

export interface TestMaskingResponse {
  original: unknown[];
  masked: unknown[];
  maskingRate: number;
}

// ============================================================================
// Audit Logs
// ============================================================================

export interface AuditLogFilters {
  startTime?: string;
  endTime?: string;
  userId?: string;
  username?: string;
  eventType?: AuditEventType;
  action?: string;
  database?: string;
  objectType?: string;
  objectName?: string;
  status?: 'success' | 'failure';
  clientAddress?: string;
  page?: number;
  pageSize?: number;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
}

export interface AuditStatistics {
  totalEvents: number;
  eventsByType: Record<AuditEventType, number>;
  eventsByStatus: {
    success: number;
    failure: number;
  };
  topUsers: Array<{
    username: string;
    eventCount: number;
  }>;
  topActions: Array<{
    action: string;
    count: number;
  }>;
  timeline: Array<{
    timestamp: string;
    count: number;
  }>;
}

export interface ExportAuditLogsRequest {
  filters: AuditLogFilters;
  format: 'csv' | 'json' | 'pdf';
  includeFields?: string[];
}

// ============================================================================
// Security Service Class
// ============================================================================

class SecurityService {
  private readonly basePath = '/security';

  // ==========================================================================
  // Security Overview
  // ==========================================================================

  async getSecurityOverview(): Promise<ApiResponse<SecurityOverview>> {
    return get<SecurityOverview>(`${this.basePath}/overview`);
  }

  async getSecurityAlerts(params?: {
    resolved?: boolean;
    severity?: string;
    type?: string;
  }): Promise<ApiResponse<SecurityAlert[]>> {
    const queryParams = params ? buildQueryParams(params) : '';
    return get<SecurityAlert[]>(`${this.basePath}/alerts${queryParams}`);
  }

  async acknowledgeAlert(alertId: string): Promise<ApiResponse<void>> {
    return post<void>(`${this.basePath}/alerts/${alertId}/acknowledge`);
  }

  async resolveAlert(alertId: string): Promise<ApiResponse<void>> {
    return post<void>(`${this.basePath}/alerts/${alertId}/resolve`);
  }

  // ==========================================================================
  // Encryption Management
  // ==========================================================================

  async getEncryptionKeys(params?: {
    status?: string;
    keyType?: KeyType;
  }): Promise<ApiResponse<EncryptionKey[]>> {
    const queryParams = params ? buildQueryParams(params) : '';
    return get<EncryptionKey[]>(`${this.basePath}/encryption/keys${queryParams}`);
  }

  async getEncryptionKey(keyId: string): Promise<ApiResponse<EncryptionKey>> {
    return get<EncryptionKey>(`${this.basePath}/encryption/keys/${keyId}`);
  }

  async createEncryptionKey(
    request: CreateEncryptionKeyRequest
  ): Promise<ApiResponse<EncryptionKey>> {
    return post<EncryptionKey>(`${this.basePath}/encryption/keys`, request);
  }

  async rotateKey(
    keyId: string,
    request?: RotateKeyRequest
  ): Promise<ApiResponse<KeyRotationStatus>> {
    return post<KeyRotationStatus>(
      `${this.basePath}/encryption/keys/${keyId}/rotate`,
      request
    );
  }

  async getKeyRotationStatus(
    keyId: string
  ): Promise<ApiResponse<KeyRotationStatus>> {
    return get<KeyRotationStatus>(
      `${this.basePath}/encryption/keys/${keyId}/rotation-status`
    );
  }

  async deactivateKey(keyId: string): Promise<ApiResponse<void>> {
    return post<void>(`${this.basePath}/encryption/keys/${keyId}/deactivate`);
  }

  async deleteKey(keyId: string): Promise<ApiResponse<void>> {
    return del<void>(`${this.basePath}/encryption/keys/${keyId}`);
  }

  async getEncryptedTables(keyId?: string): Promise<ApiResponse<EncryptedTable[]>> {
    const queryParams = keyId ? buildQueryParams({ keyId }) : '';
    return get<EncryptedTable[]>(`${this.basePath}/encryption/tables${queryParams}`);
  }

  async encryptTable(
    schema: string,
    table: string,
    columns: string[],
    keyId: string
  ): Promise<ApiResponse<void>> {
    return post<void>(`${this.basePath}/encryption/encrypt`, {
      schema,
      table,
      columns,
      keyId,
    });
  }

  async decryptTable(
    schema: string,
    table: string,
    columns: string[]
  ): Promise<ApiResponse<void>> {
    return post<void>(`${this.basePath}/encryption/decrypt`, {
      schema,
      table,
      columns,
    });
  }

  // ==========================================================================
  // Data Masking
  // ==========================================================================

  async getMaskingPolicies(params?: {
    table?: string;
    isEnabled?: boolean;
  }): Promise<ApiResponse<DataMaskingPolicy[]>> {
    const queryParams = params ? buildQueryParams(params) : '';
    return get<DataMaskingPolicy[]>(`${this.basePath}/masking/policies${queryParams}`);
  }

  async getMaskingPolicy(policyId: string): Promise<ApiResponse<DataMaskingPolicy>> {
    return get<DataMaskingPolicy>(`${this.basePath}/masking/policies/${policyId}`);
  }

  async createMaskingPolicy(
    request: CreateMaskingPolicyRequest
  ): Promise<ApiResponse<DataMaskingPolicy>> {
    return post<DataMaskingPolicy>(`${this.basePath}/masking/policies`, request);
  }

  async updateMaskingPolicy(
    policyId: string,
    request: UpdateMaskingPolicyRequest
  ): Promise<ApiResponse<DataMaskingPolicy>> {
    return put<DataMaskingPolicy>(
      `${this.basePath}/masking/policies/${policyId}`,
      request
    );
  }

  async deleteMaskingPolicy(policyId: string): Promise<ApiResponse<void>> {
    return del<void>(`${this.basePath}/masking/policies/${policyId}`);
  }

  async toggleMaskingPolicy(
    policyId: string,
    enabled: boolean
  ): Promise<ApiResponse<DataMaskingPolicy>> {
    return post<DataMaskingPolicy>(
      `${this.basePath}/masking/policies/${policyId}/toggle`,
      { enabled }
    );
  }

  async testMasking(
    request: TestMaskingRequest
  ): Promise<ApiResponse<TestMaskingResponse>> {
    return post<TestMaskingResponse>(`${this.basePath}/masking/test`, request);
  }

  async getMaskingPreview(
    table: string,
    column: string,
    maskingType: MaskingType,
    limit?: number
  ): Promise<ApiResponse<{ original: unknown[]; masked: unknown[] }>> {
    return post<{ original: unknown[]; masked: unknown[] }>(
      `${this.basePath}/masking/preview`,
      {
        table,
        column,
        maskingType,
        limit: limit || 10,
      }
    );
  }

  // ==========================================================================
  // Audit Logs
  // ==========================================================================

  async getAuditLogs(
    filters: AuditLogFilters
  ): Promise<ApiResponse<PaginatedResponse<AuditLog>>> {
    const queryParams = buildQueryParams(filters);
    return get<PaginatedResponse<AuditLog>>(`${this.basePath}/audit/logs${queryParams}`);
  }

  async getAuditLog(logId: string): Promise<ApiResponse<AuditLog>> {
    return get<AuditLog>(`${this.basePath}/audit/logs/${logId}`);
  }

  async getAuditStatistics(
    startTime?: string,
    endTime?: string
  ): Promise<ApiResponse<AuditStatistics>> {
    const queryParams = buildQueryParams({ startTime, endTime });
    return get<AuditStatistics>(`${this.basePath}/audit/statistics${queryParams}`);
  }

  async exportAuditLogs(
    request: ExportAuditLogsRequest
  ): Promise<ApiResponse<{ downloadUrl: string }>> {
    return post<{ downloadUrl: string }>(`${this.basePath}/audit/export`, request);
  }

  async getAuditEventTypes(): Promise<ApiResponse<AuditEventType[]>> {
    return get<AuditEventType[]>(`${this.basePath}/audit/event-types`);
  }

  // ==========================================================================
  // Security Features Status
  // ==========================================================================

  async getSecurityFeatures(): Promise<
    ApiResponse<{
      overall_status: string;
      features: Record<
        string,
        {
          enabled: boolean;
          status: string;
          description: string;
          last_check: number;
        }
      >;
      enabled_count: number;
      active_count: number;
      total_count: number;
      compliance_standards: string[];
      last_security_audit: number;
    }>
  > {
    return get(`${this.basePath}/features`);
  }

  // ==========================================================================
  // Compliance & Security Scanning
  // ==========================================================================

  async runSecurityScan(): Promise<ApiResponse<{ scanId: string }>> {
    return post<{ scanId: string }>(`${this.basePath}/scan`);
  }

  async getSecurityScanResults(scanId: string): Promise<
    ApiResponse<{
      scanId: string;
      status: 'running' | 'completed' | 'failed';
      findings: Array<{
        severity: 'critical' | 'high' | 'medium' | 'low';
        category: string;
        description: string;
        recommendation: string;
      }>;
      complianceScore: number;
      timestamp: string;
    }>
  > {
    return get(`${this.basePath}/scan/${scanId}`);
  }

  async getComplianceReport(
    standard: 'SOC2' | 'HIPAA' | 'GDPR' | 'PCI-DSS'
  ): Promise<
    ApiResponse<{
      standard: string;
      score: number;
      requirements: Array<{
        id: string;
        name: string;
        status: 'compliant' | 'non_compliant' | 'partial';
        details: string;
      }>;
      generatedAt: string;
    }>
  > {
    return get(`${this.basePath}/compliance/${standard}`);
  }
}

// ============================================================================
// Export Singleton Instance
// ============================================================================

export const securityService = new SecurityService();
export default securityService;
