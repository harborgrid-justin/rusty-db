// ============================================================================
// Security Hooks - React hooks for security data management
// ============================================================================

import { useState, useEffect, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  securityService,
  type SecurityOverview,
  type SecurityAlert,
  type CreateEncryptionKeyRequest,
  type RotateKeyRequest,
  type KeyRotationStatus,
  type EncryptedTable,
  type CreateMaskingPolicyRequest,
  type UpdateMaskingPolicyRequest,
  type TestMaskingRequest,
  type TestMaskingResponse,
  type AuditLogFilters,
  type AuditStatistics,
} from '../services/securityService';
import type {
  EncryptionKey,
  DataMaskingPolicy,
  AuditLog,
  PaginatedResponse,
  KeyType,
  MaskingType,
} from '@/types';
import { getErrorMessage } from '../services/api';

// ============================================================================
// Security Overview Hook
// ============================================================================

export function useSecurityOverview() {
  return useQuery<SecurityOverview>({
    queryKey: ['security', 'overview'],
    queryFn: async () => {
      const response = await securityService.getSecurityOverview();
      if (!response.data) {
        throw new Error('Failed to fetch security overview');
      }
      return response.data;
    },
    refetchInterval: 30000, // Refresh every 30 seconds
  });
}

export function useSecurityAlerts(params?: {
  resolved?: boolean;
  severity?: string;
  type?: string;
}) {
  return useQuery<SecurityAlert[]>({
    queryKey: ['security', 'alerts', params],
    queryFn: async () => {
      const response = await securityService.getSecurityAlerts(params);
      if (!response.data) {
        throw new Error('Failed to fetch security alerts');
      }
      return response.data;
    },
    refetchInterval: 10000, // Refresh every 10 seconds
  });
}

export function useAcknowledgeAlert() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (alertId: string) => securityService.acknowledgeAlert(alertId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'alerts'] });
    },
  });
}

export function useResolveAlert() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (alertId: string) => securityService.resolveAlert(alertId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'alerts'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

// ============================================================================
// Encryption Management Hooks
// ============================================================================

export function useEncryptionKeys(params?: { status?: string; keyType?: KeyType }) {
  return useQuery<EncryptionKey[]>({
    queryKey: ['security', 'encryption', 'keys', params],
    queryFn: async () => {
      const response = await securityService.getEncryptionKeys(params);
      if (!response.data) {
        throw new Error('Failed to fetch encryption keys');
      }
      return response.data;
    },
  });
}

export function useEncryptionKey(keyId: string | null) {
  return useQuery<EncryptionKey>({
    queryKey: ['security', 'encryption', 'keys', keyId],
    queryFn: async () => {
      if (!keyId) throw new Error('Key ID is required');
      const response = await securityService.getEncryptionKey(keyId);
      if (!response.data) {
        throw new Error('Failed to fetch encryption key');
      }
      return response.data;
    },
    enabled: !!keyId,
  });
}

export function useCreateEncryptionKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateEncryptionKeyRequest) =>
      securityService.createEncryptionKey(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'encryption', 'keys'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useRotateKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ keyId, request }: { keyId: string; request?: RotateKeyRequest }) =>
      securityService.rotateKey(keyId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'encryption', 'keys'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useKeyRotationStatus(keyId: string | null) {
  return useQuery<KeyRotationStatus>({
    queryKey: ['security', 'encryption', 'rotation', keyId],
    queryFn: async () => {
      if (!keyId) throw new Error('Key ID is required');
      const response = await securityService.getKeyRotationStatus(keyId);
      if (!response.data) {
        throw new Error('Failed to fetch rotation status');
      }
      return response.data;
    },
    enabled: !!keyId,
    refetchInterval: (data) => {
      // Poll while rotation is in progress
      if (data?.status === 'in_progress' || data?.status === 'pending') {
        return 2000;
      }
      return false;
    },
  });
}

export function useDeactivateKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (keyId: string) => securityService.deactivateKey(keyId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'encryption', 'keys'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (keyId: string) => securityService.deleteKey(keyId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'encryption', 'keys'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useEncryptedTables(keyId?: string) {
  return useQuery<EncryptedTable[]>({
    queryKey: ['security', 'encryption', 'tables', keyId],
    queryFn: async () => {
      const response = await securityService.getEncryptedTables(keyId);
      if (!response.data) {
        throw new Error('Failed to fetch encrypted tables');
      }
      return response.data;
    },
  });
}

export function useEncryptTable() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      schema,
      table,
      columns,
      keyId,
    }: {
      schema: string;
      table: string;
      columns: string[];
      keyId: string;
    }) => securityService.encryptTable(schema, table, columns, keyId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'encryption', 'tables'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useDecryptTable() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      schema,
      table,
      columns,
    }: {
      schema: string;
      table: string;
      columns: string[];
    }) => securityService.decryptTable(schema, table, columns),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'encryption', 'tables'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

// ============================================================================
// Data Masking Hooks
// ============================================================================

export function useMaskingPolicies(params?: { table?: string; isEnabled?: boolean }) {
  return useQuery<DataMaskingPolicy[]>({
    queryKey: ['security', 'masking', 'policies', params],
    queryFn: async () => {
      const response = await securityService.getMaskingPolicies(params);
      if (!response.data) {
        throw new Error('Failed to fetch masking policies');
      }
      return response.data;
    },
  });
}

export function useMaskingPolicy(policyId: string | null) {
  return useQuery<DataMaskingPolicy>({
    queryKey: ['security', 'masking', 'policies', policyId],
    queryFn: async () => {
      if (!policyId) throw new Error('Policy ID is required');
      const response = await securityService.getMaskingPolicy(policyId);
      if (!response.data) {
        throw new Error('Failed to fetch masking policy');
      }
      return response.data;
    },
    enabled: !!policyId,
  });
}

export function useCreateMaskingPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateMaskingPolicyRequest) =>
      securityService.createMaskingPolicy(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'masking', 'policies'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useUpdateMaskingPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      policyId,
      request,
    }: {
      policyId: string;
      request: UpdateMaskingPolicyRequest;
    }) => securityService.updateMaskingPolicy(policyId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'masking', 'policies'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useDeleteMaskingPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (policyId: string) => securityService.deleteMaskingPolicy(policyId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'masking', 'policies'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useToggleMaskingPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ policyId, enabled }: { policyId: string; enabled: boolean }) =>
      securityService.toggleMaskingPolicy(policyId, enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'masking', 'policies'] });
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useTestMasking() {
  return useMutation<TestMaskingResponse, Error, TestMaskingRequest>({
    mutationFn: async (request: TestMaskingRequest) => {
      const response = await securityService.testMasking(request);
      if (!response.data) {
        throw new Error('Failed to test masking');
      }
      return response.data;
    },
  });
}

export function useMaskingPreview() {
  return useMutation({
    mutationFn: ({
      table,
      column,
      maskingType,
      limit,
    }: {
      table: string;
      column: string;
      maskingType: MaskingType;
      limit?: number;
    }) => securityService.getMaskingPreview(table, column, maskingType, limit),
  });
}

// ============================================================================
// Audit Log Hooks
// ============================================================================

export function useAuditLogs(filters: AuditLogFilters) {
  return useQuery<PaginatedResponse<AuditLog>>({
    queryKey: ['security', 'audit', 'logs', filters],
    queryFn: async () => {
      const response = await securityService.getAuditLogs(filters);
      if (!response.data) {
        throw new Error('Failed to fetch audit logs');
      }
      return response.data;
    },
    keepPreviousData: true,
  });
}

export function useAuditLog(logId: string | null) {
  return useQuery<AuditLog>({
    queryKey: ['security', 'audit', 'logs', logId],
    queryFn: async () => {
      if (!logId) throw new Error('Log ID is required');
      const response = await securityService.getAuditLog(logId);
      if (!response.data) {
        throw new Error('Failed to fetch audit log');
      }
      return response.data;
    },
    enabled: !!logId,
  });
}

export function useAuditStatistics(startTime?: string, endTime?: string) {
  return useQuery<AuditStatistics>({
    queryKey: ['security', 'audit', 'statistics', startTime, endTime],
    queryFn: async () => {
      const response = await securityService.getAuditStatistics(startTime, endTime);
      if (!response.data) {
        throw new Error('Failed to fetch audit statistics');
      }
      return response.data;
    },
  });
}

export function useExportAuditLogs() {
  return useMutation({
    mutationFn: (request: {
      filters: AuditLogFilters;
      format: 'csv' | 'json' | 'pdf';
      includeFields?: string[];
    }) => securityService.exportAuditLogs(request),
  });
}

// ============================================================================
// Security Scanning Hooks
// ============================================================================

export function useRunSecurityScan() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => securityService.runSecurityScan(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'overview'] });
    },
  });
}

export function useSecurityScanResults(scanId: string | null) {
  return useQuery({
    queryKey: ['security', 'scan', scanId],
    queryFn: async () => {
      if (!scanId) throw new Error('Scan ID is required');
      const response = await securityService.getSecurityScanResults(scanId);
      if (!response.data) {
        throw new Error('Failed to fetch scan results');
      }
      return response.data;
    },
    enabled: !!scanId,
    refetchInterval: (data) => {
      // Poll while scan is running
      if (data?.status === 'running') {
        return 5000;
      }
      return false;
    },
  });
}

export function useComplianceReport(
  standard: 'SOC2' | 'HIPAA' | 'GDPR' | 'PCI-DSS' | null
) {
  return useQuery({
    queryKey: ['security', 'compliance', standard],
    queryFn: async () => {
      if (!standard) throw new Error('Compliance standard is required');
      const response = await securityService.getComplianceReport(standard);
      if (!response.data) {
        throw new Error('Failed to fetch compliance report');
      }
      return response.data;
    },
    enabled: !!standard,
  });
}

// ============================================================================
// Combined Security Data Hook
// ============================================================================

export function useSecurityData() {
  const overview = useSecurityOverview();
  const alerts = useSecurityAlerts({ resolved: false });
  const keys = useEncryptionKeys();
  const policies = useMaskingPolicies();

  return {
    overview: overview.data,
    alerts: alerts.data,
    keys: keys.data,
    policies: policies.data,
    isLoading:
      overview.isLoading || alerts.isLoading || keys.isLoading || policies.isLoading,
    error:
      overview.error || alerts.error || keys.error || policies.error
        ? getErrorMessage(
            overview.error || alerts.error || keys.error || policies.error
          )
        : null,
  };
}
