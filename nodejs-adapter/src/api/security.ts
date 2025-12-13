/**
 * RustyDB Security Layer API Client
 *
 * Comprehensive TypeScript adapter for all Security Layer REST endpoints including:
 * - Transparent Data Encryption (TDE)
 * - Data Masking Policies
 * - Virtual Private Database (VPD)
 * - Role-Based Access Control (RBAC)
 * - Audit Logging and Compliance
 * - Insider Threat Detection
 * - Enterprise Authentication (LDAP, OAuth, SSO)
 *
 * @module security
 */

import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';

// ============================================================================
// Encryption Types
// ============================================================================

/**
 * Encryption status response
 */
export interface EncryptionStatus {
    tde_enabled: boolean;
    default_algorithm: string;
    encrypted_tablespaces: TablespaceEncryption[];
    encrypted_columns: ColumnEncryption[];
    total_bytes_encrypted: number;
    total_bytes_decrypted: number;
    key_rotation_status: KeyRotationStatus;
}

/**
 * Tablespace encryption information
 */
export interface TablespaceEncryption {
    tablespace_name: string;
    algorithm: string;
    key_id: string;
    key_version: number;
    enabled: boolean;
    created_at: number;
}

/**
 * Column encryption information
 */
export interface ColumnEncryption {
    table_name: string;
    column_name: string;
    algorithm: string;
    key_id: string;
    enabled: boolean;
}

/**
 * Key rotation status
 */
export interface KeyRotationStatus {
    last_rotation?: number;
    next_scheduled_rotation?: number;
    keys_rotated_total: number;
}

/**
 * Enable TDE request
 */
export interface EnableEncryptionRequest {
    tablespace_name: string;
    algorithm: string;
    compress_before_encrypt?: boolean;
}

/**
 * Enable column encryption request
 */
export interface EnableColumnEncryptionRequest {
    table_name: string;
    column_name: string;
    algorithm: string;
}

/**
 * DDL operation result
 */
export interface DdlResult {
    success: boolean;
    message: string;
    affected_objects: string[];
}

/**
 * Key generation request
 */
export interface KeyGenerationRequest {
    key_type: string;
    algorithm: string;
    key_name: string;
}

/**
 * Key generation/rotation result
 */
export interface KeyResult {
    success: boolean;
    key_id: string;
    key_version: number;
    algorithm: string;
    created_at: number;
}

// ============================================================================
// Data Masking Types
// ============================================================================

/**
 * Masking policy response
 */
export interface MaskingPolicyResponse {
    name: string;
    column_pattern: string;
    table_pattern?: string;
    masking_type: string;
    enabled: boolean;
    priority: number;
    created_at: number;
}

/**
 * Create masking policy request
 */
export interface CreateMaskingPolicy {
    name: string;
    column_pattern: string;
    table_pattern?: string;
    masking_type: string;
    priority?: number;
    consistency_key?: string;
}

/**
 * Update masking policy request
 */
export interface UpdateMaskingPolicy {
    enabled?: boolean;
    priority?: number;
    masking_type?: string;
}

/**
 * Test masking request
 */
export interface MaskingTest {
    policy_name: string;
    test_values: string[];
}

/**
 * Test masking result
 */
export interface MaskingTestResult {
    policy_name: string;
    results: MaskingTestCase[];
}

/**
 * Individual masking test case
 */
export interface MaskingTestCase {
    original: string;
    masked: string;
    masking_type: string;
}

// ============================================================================
// Virtual Private Database (VPD) Types
// ============================================================================

/**
 * VPD policy response
 */
export interface VpdPolicyResponse {
    name: string;
    table_name: string;
    schema_name?: string;
    predicate: string;
    policy_scope: string;
    enabled: boolean;
    created_at: number;
}

/**
 * Create VPD policy request
 */
export interface CreateVpdPolicy {
    name: string;
    table_name: string;
    schema_name?: string;
    predicate: string;
    policy_scope?: string;
}

/**
 * Update VPD policy request
 */
export interface UpdateVpdPolicy {
    enabled?: boolean;
    predicate?: string;
    policy_scope?: string;
}

/**
 * Test VPD predicate request
 */
export interface TestVpdPredicate {
    predicate: string;
    context: Record<string, string>;
}

/**
 * Test VPD predicate result
 */
export interface TestVpdPredicateResult {
    original_predicate: string;
    evaluated_predicate: string;
    context_used: Record<string, string>;
    valid_sql: boolean;
}

// ============================================================================
// RBAC Types
// ============================================================================

/**
 * Create role request
 */
export interface CreateRoleRequest {
    id: string;
    name: string;
    description?: string;
    parent_roles?: string[];
    permissions?: string[];
}

/**
 * Update role request
 */
export interface UpdateRoleRequest {
    name?: string;
    description?: string;
    parent_roles?: string[];
    permissions?: string[];
    is_active?: boolean;
}

/**
 * Role response
 */
export interface RoleResponse {
    id: string;
    name: string;
    description?: string;
    parent_roles: string[];
    permissions: string[];
    is_active: boolean;
    created_at: number;
    updated_at: number;
    owner?: string;
    priority: number;
}

/**
 * Assign permissions request
 */
export interface AssignPermissionsRequest {
    permissions: string[];
}

/**
 * Permission response
 */
export interface PermissionResponse {
    id: string;
    name: string;
    description: string;
    category: string;
}

// ============================================================================
// Threat Detection Types
// ============================================================================

/**
 * Threat status response
 */
export interface ThreatStatusResponse {
    enabled: boolean;
    total_assessments: number;
    critical_threats: number;
    high_threats: number;
    blocked_queries: number;
    exfiltration_attempts: number;
    escalation_attempts: number;
    baselines_established: number;
}

/**
 * Threat history item
 */
export interface ThreatHistoryItem {
    assessment_id: string;
    user_id: string;
    query_text: string;
    total_score: number;
    threat_level: string;
    risk_factors: string[];
    timestamp: number;
    action: string;
    client_ip?: string;
    location?: string;
}

/**
 * Insider threat status response
 */
export interface InsiderThreatStatusResponse {
    enabled: boolean;
    auto_block_critical: boolean;
    require_mfa_high_risk: boolean;
    alert_threshold: number;
    block_threshold: number;
    behavioral_analytics_enabled: boolean;
    anomaly_detection_enabled: boolean;
    exfiltration_prevention_enabled: boolean;
    escalation_detection_enabled: boolean;
}

// ============================================================================
// Audit Types
// ============================================================================

/**
 * Audit query parameters
 */
export interface AuditQueryParams {
    start_time?: number;
    end_time?: number;
    user_id?: string;
    action?: string;
    object_name?: string;
    session_id?: string;
    limit?: number;
    offset?: number;
}

/**
 * Audit entry
 */
export interface AuditEntry {
    id: number;
    timestamp: number;
    user_id: string;
    session_id: string;
    client_ip: string;
    action: string;
    object_name?: string;
    statement?: string;
    success: boolean;
    error_message?: string;
    execution_time_ms?: number;
}

/**
 * Audit export configuration
 */
export interface AuditExportConfig {
    format: string;
    start_time: number;
    end_time: number;
    destination: string;
    include_sensitive?: boolean;
}

/**
 * Export result
 */
export interface ExportResult {
    success: boolean;
    records_exported: number;
    file_path: string;
    file_size_bytes: number;
    checksum: string;
}

/**
 * Compliance report parameters
 */
export interface ComplianceParams {
    regulation: string;
    start_date: number;
    end_date: number;
    include_recommendations?: boolean;
}

/**
 * Compliance report response
 */
export interface ComplianceReportResponse {
    regulation: string;
    period_start: number;
    period_end: number;
    compliant: boolean;
    total_audit_records: number;
    violations: ComplianceViolation[];
    recommendations: string[];
    generated_at: number;
}

/**
 * Compliance violation
 */
export interface ComplianceViolation {
    violation_type: string;
    severity: string;
    description: string;
    affected_records: number[];
    remediation: string;
}

// ============================================================================
// Network Security Types
// ============================================================================

/**
 * Circuit breaker status
 */
export interface CircuitBreakerStatus {
    name: string;
    state: string;
    failure_count: number;
    success_count: number;
    last_failure?: number;
    last_state_change: number;
    failure_threshold: number;
    timeout_secs: number;
}

// ============================================================================
// Enterprise Authentication Types
// ============================================================================

/**
 * LDAP configuration
 */
export interface LdapConfig {
    enabled: boolean;
    server_url: string;
    bind_dn: string;
    bind_password: string;
    base_dn: string;
    user_filter: string;
    group_filter?: string;
    use_tls: boolean;
    verify_certificate: boolean;
    timeout_secs: number;
}

/**
 * OAuth provider configuration
 */
export interface OAuthProviderConfig {
    provider_name: string;
    provider_type: string;
    client_id: string;
    client_secret: string;
    authorization_endpoint: string;
    token_endpoint: string;
    user_info_endpoint: string;
    scopes: string[];
    enabled: boolean;
}

/**
 * OAuth provider list response
 */
export interface OAuthProviderList {
    providers: OAuthProviderInfo[];
    total_count: number;
}

/**
 * OAuth provider info
 */
export interface OAuthProviderInfo {
    provider_id: string;
    provider_name: string;
    provider_type: string;
    enabled: boolean;
    configured: boolean;
}

/**
 * SSO (SAML) configuration
 */
export interface SsoConfig {
    enabled: boolean;
    entity_id: string;
    sso_url: string;
    slo_url?: string;
    certificate: string;
    private_key: string;
    idp_entity_id: string;
    idp_sso_url: string;
    idp_certificate: string;
    name_id_format: string;
    attributes_mapping: Record<string, string>;
}

/**
 * SAML metadata response
 */
export interface SamlMetadata {
    entity_id: string;
    sso_url: string;
    slo_url?: string;
    certificate: string;
    metadata_xml: string;
}

/**
 * Test connection result
 */
export interface TestConnectionResult {
    success: boolean;
    message: string;
    details?: any;
    response_time_ms: number;
}

// ============================================================================
// Common Types
// ============================================================================

/**
 * Generic success response
 */
export interface SuccessResponse {
    success: boolean;
    message: string;
}

/**
 * API Error
 */
export interface ApiError {
    code: string;
    message: string;
}

// ============================================================================
// Security Client Configuration
// ============================================================================

/**
 * Security client configuration options
 */
export interface SecurityClientConfig {
    baseUrl?: string;
    timeout?: number;
    headers?: Record<string, string>;
}

// ============================================================================
// RustyDB Security API Client
// ============================================================================

/**
 * Comprehensive security API client for RustyDB
 */
export class RustyDBSecurityClient {
    private client: AxiosInstance;
    private baseUrl: string;

    /**
     * Create a new security client
     * @param config Client configuration
     */
    constructor(config: SecurityClientConfig = {}) {
        this.baseUrl = config.baseUrl || 'http://localhost:8080';

        this.client = axios.create({
            baseURL: this.baseUrl,
            timeout: config.timeout || 30000,
            headers: {
                'Content-Type': 'application/json',
                ...config.headers,
            },
        });
    }

    // ========================================================================
    // Encryption Methods
    // ========================================================================

    /**
     * Get current encryption status
     * @returns Encryption status including TDE configuration and statistics
     */
    async getEncryptionStatus(): Promise<EncryptionStatus> {
        const response = await this.client.get<EncryptionStatus>('/api/v1/security/encryption/status');
        return response.data;
    }

    /**
     * Enable transparent data encryption for a tablespace
     * @param request Encryption configuration
     * @returns DDL operation result
     */
    async enableEncryption(request: EnableEncryptionRequest): Promise<DdlResult> {
        const response = await this.client.post<DdlResult>('/api/v1/security/encryption/enable', request);
        return response.data;
    }

    /**
     * Enable column-level encryption
     * @param request Column encryption configuration
     * @returns DDL operation result
     */
    async enableColumnEncryption(request: EnableColumnEncryptionRequest): Promise<DdlResult> {
        const response = await this.client.post<DdlResult>('/api/v1/security/encryption/column', request);
        return response.data;
    }

    /**
     * Generate a new encryption key
     * @param request Key generation parameters
     * @returns Key generation result
     */
    async generateKey(request: KeyGenerationRequest): Promise<KeyResult> {
        const response = await this.client.post<KeyResult>('/api/v1/security/keys/generate', request);
        return response.data;
    }

    /**
     * Rotate an encryption key
     * @param keyId Key ID to rotate
     * @returns Key rotation result
     */
    async rotateKey(keyId: string): Promise<KeyResult> {
        const response = await this.client.post<KeyResult>(`/api/v1/security/keys/${encodeURIComponent(keyId)}/rotate`);
        return response.data;
    }

    /**
     * List all encryption keys
     * @returns List of encryption keys
     */
    async listKeys(): Promise<KeyResult[]> {
        const response = await this.client.get<KeyResult[]>('/api/v1/security/keys');
        return response.data;
    }

    // ========================================================================
    // Data Masking Methods
    // ========================================================================

    /**
     * List all masking policies
     * @returns List of masking policies
     */
    async listMaskingPolicies(): Promise<MaskingPolicyResponse[]> {
        const response = await this.client.get<MaskingPolicyResponse[]>('/api/v1/security/masking/policies');
        return response.data;
    }

    /**
     * Get a specific masking policy
     * @param name Policy name
     * @returns Masking policy details
     */
    async getMaskingPolicy(name: string): Promise<MaskingPolicyResponse> {
        const response = await this.client.get<MaskingPolicyResponse>(
            `/api/v1/security/masking/policies/${encodeURIComponent(name)}`
        );
        return response.data;
    }

    /**
     * Create a new masking policy
     * @param request Policy configuration
     * @returns Created policy
     */
    async createMaskingPolicy(request: CreateMaskingPolicy): Promise<MaskingPolicyResponse> {
        const response = await this.client.post<MaskingPolicyResponse>('/api/v1/security/masking/policies', request);
        return response.data;
    }

    /**
     * Update an existing masking policy
     * @param name Policy name
     * @param request Update parameters
     * @returns Updated policy
     */
    async updateMaskingPolicy(name: string, request: UpdateMaskingPolicy): Promise<MaskingPolicyResponse> {
        const response = await this.client.put<MaskingPolicyResponse>(
            `/api/v1/security/masking/policies/${encodeURIComponent(name)}`,
            request
        );
        return response.data;
    }

    /**
     * Delete a masking policy
     * @param name Policy name
     */
    async deleteMaskingPolicy(name: string): Promise<SuccessResponse> {
        const response = await this.client.delete<SuccessResponse>(
            `/api/v1/security/masking/policies/${encodeURIComponent(name)}`
        );
        return response.data;
    }

    /**
     * Test masking policy against sample data
     * @param request Test configuration
     * @returns Test results
     */
    async testMasking(request: MaskingTest): Promise<MaskingTestResult> {
        const response = await this.client.post<MaskingTestResult>('/api/v1/security/masking/test', request);
        return response.data;
    }

    /**
     * Enable a masking policy
     * @param name Policy name
     */
    async enableMaskingPolicy(name: string): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>(
            `/api/v1/security/masking/policies/${encodeURIComponent(name)}/enable`
        );
        return response.data;
    }

    /**
     * Disable a masking policy
     * @param name Policy name
     */
    async disableMaskingPolicy(name: string): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>(
            `/api/v1/security/masking/policies/${encodeURIComponent(name)}/disable`
        );
        return response.data;
    }

    // ========================================================================
    // Virtual Private Database (VPD) Methods
    // ========================================================================

    /**
     * List all VPD policies
     * @returns List of VPD policies
     */
    async listVpdPolicies(): Promise<VpdPolicyResponse[]> {
        const response = await this.client.get<VpdPolicyResponse[]>('/api/v1/security/vpd/policies');
        return response.data;
    }

    /**
     * Get a specific VPD policy
     * @param name Policy name
     * @returns VPD policy details
     */
    async getVpdPolicy(name: string): Promise<VpdPolicyResponse> {
        const response = await this.client.get<VpdPolicyResponse>(
            `/api/v1/security/vpd/policies/${encodeURIComponent(name)}`
        );
        return response.data;
    }

    /**
     * Create a new VPD policy
     * @param request Policy configuration
     * @returns Created policy
     */
    async createVpdPolicy(request: CreateVpdPolicy): Promise<VpdPolicyResponse> {
        const response = await this.client.post<VpdPolicyResponse>('/api/v1/security/vpd/policies', request);
        return response.data;
    }

    /**
     * Update an existing VPD policy
     * @param name Policy name
     * @param request Update parameters
     * @returns Updated policy
     */
    async updateVpdPolicy(name: string, request: UpdateVpdPolicy): Promise<VpdPolicyResponse> {
        const response = await this.client.put<VpdPolicyResponse>(
            `/api/v1/security/vpd/policies/${encodeURIComponent(name)}`,
            request
        );
        return response.data;
    }

    /**
     * Delete a VPD policy
     * @param name Policy name
     */
    async deleteVpdPolicy(name: string): Promise<SuccessResponse> {
        const response = await this.client.delete<SuccessResponse>(
            `/api/v1/security/vpd/policies/${encodeURIComponent(name)}`
        );
        return response.data;
    }

    /**
     * Test a VPD predicate with sample context
     * @param request Predicate test configuration
     * @returns Test result
     */
    async testVpdPredicate(request: TestVpdPredicate): Promise<TestVpdPredicateResult> {
        const response = await this.client.post<TestVpdPredicateResult>('/api/v1/security/vpd/test-predicate', request);
        return response.data;
    }

    /**
     * Get all VPD policies for a specific table
     * @param tableName Table name
     * @returns List of policies for the table
     */
    async getTablePolicies(tableName: string): Promise<VpdPolicyResponse[]> {
        const response = await this.client.get<VpdPolicyResponse[]>(
            `/api/v1/security/vpd/policies/table/${encodeURIComponent(tableName)}`
        );
        return response.data;
    }

    /**
     * Enable a VPD policy
     * @param name Policy name
     */
    async enableVpdPolicy(name: string): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>(
            `/api/v1/security/vpd/policies/${encodeURIComponent(name)}/enable`
        );
        return response.data;
    }

    /**
     * Disable a VPD policy
     * @param name Policy name
     */
    async disableVpdPolicy(name: string): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>(
            `/api/v1/security/vpd/policies/${encodeURIComponent(name)}/disable`
        );
        return response.data;
    }

    // ========================================================================
    // RBAC Methods
    // ========================================================================

    /**
     * List all roles
     * @returns List of roles
     */
    async listRoles(): Promise<RoleResponse[]> {
        const response = await this.client.get<RoleResponse[]>('/api/v1/security/roles');
        return response.data;
    }

    /**
     * Create a new role
     * @param request Role configuration
     * @returns Created role
     */
    async createRole(request: CreateRoleRequest): Promise<RoleResponse> {
        const response = await this.client.post<RoleResponse>('/api/v1/security/roles', request);
        return response.data;
    }

    /**
     * Get a specific role
     * @param roleId Role ID
     * @returns Role details
     */
    async getRole(roleId: string): Promise<RoleResponse> {
        const response = await this.client.get<RoleResponse>(
            `/api/v1/security/roles/${encodeURIComponent(roleId)}`
        );
        return response.data;
    }

    /**
     * Update an existing role
     * @param roleId Role ID
     * @param request Update parameters
     * @returns Updated role
     */
    async updateRole(roleId: string, request: UpdateRoleRequest): Promise<RoleResponse> {
        const response = await this.client.put<RoleResponse>(
            `/api/v1/security/roles/${encodeURIComponent(roleId)}`,
            request
        );
        return response.data;
    }

    /**
     * Delete a role
     * @param roleId Role ID
     */
    async deleteRole(roleId: string): Promise<SuccessResponse> {
        const response = await this.client.delete<SuccessResponse>(
            `/api/v1/security/roles/${encodeURIComponent(roleId)}`
        );
        return response.data;
    }

    /**
     * List all available permissions
     * @returns List of permissions
     */
    async listPermissions(): Promise<PermissionResponse[]> {
        const response = await this.client.get<PermissionResponse[]>('/api/v1/security/permissions');
        return response.data;
    }

    /**
     * Assign permissions to a role
     * @param roleId Role ID
     * @param request Permissions to assign
     * @returns Updated role
     */
    async assignPermissions(roleId: string, request: AssignPermissionsRequest): Promise<RoleResponse> {
        const response = await this.client.post<RoleResponse>(
            `/api/v1/security/roles/${encodeURIComponent(roleId)}/permissions`,
            request
        );
        return response.data;
    }

    // ========================================================================
    // Threat Detection Methods
    // ========================================================================

    /**
     * Get current threat detection status
     * @returns Threat detection statistics
     */
    async getThreatStatus(): Promise<ThreatStatusResponse> {
        const response = await this.client.get<ThreatStatusResponse>('/api/v1/security/threats');
        return response.data;
    }

    /**
     * Get threat detection history
     * @returns List of recent threat assessments
     */
    async getThreatHistory(): Promise<ThreatHistoryItem[]> {
        const response = await this.client.get<ThreatHistoryItem[]>('/api/v1/security/threats/history');
        return response.data;
    }

    /**
     * Get insider threat detection configuration
     * @returns Insider threat status
     */
    async getInsiderThreatStatus(): Promise<InsiderThreatStatusResponse> {
        const response = await this.client.get<InsiderThreatStatusResponse>('/api/v1/security/insider-threats');
        return response.data;
    }

    // ========================================================================
    // Audit Methods
    // ========================================================================

    /**
     * Query audit logs
     * @param params Query parameters
     * @returns List of audit entries
     */
    async queryAuditLogs(params?: AuditQueryParams): Promise<AuditEntry[]> {
        const response = await this.client.get<AuditEntry[]>('/api/v1/security/audit/logs', { params });
        return response.data;
    }

    /**
     * Export audit logs
     * @param config Export configuration
     * @returns Export result
     */
    async exportAuditLogs(config: AuditExportConfig): Promise<ExportResult> {
        const response = await this.client.post<ExportResult>('/api/v1/security/audit/export', config);
        return response.data;
    }

    /**
     * Generate compliance report
     * @param params Compliance report parameters
     * @returns Compliance report
     */
    async complianceReport(params: ComplianceParams): Promise<ComplianceReportResponse> {
        const response = await this.client.get<ComplianceReportResponse>('/api/v1/security/audit/compliance', { params });
        return response.data;
    }

    /**
     * Get audit statistics
     * @returns Audit statistics
     */
    async getAuditStats(): Promise<any> {
        const response = await this.client.get('/api/v1/security/audit/stats');
        return response.data;
    }

    /**
     * Verify audit log integrity
     * @returns Integrity verification result
     */
    async verifyAuditIntegrity(): Promise<any> {
        const response = await this.client.post('/api/v1/security/audit/verify');
        return response.data;
    }

    // ========================================================================
    // Network Security Methods
    // ========================================================================

    /**
     * Get circuit breaker status
     * @returns List of circuit breakers
     */
    async getCircuitBreakers(): Promise<CircuitBreakerStatus[]> {
        const response = await this.client.get<CircuitBreakerStatus[]>('/api/v1/network/circuit-breakers');
        return response.data;
    }

    // ========================================================================
    // Enterprise Authentication Methods
    // ========================================================================

    /**
     * Configure LDAP authentication
     * @param config LDAP configuration
     */
    async configureLdap(config: LdapConfig): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>('/api/v1/auth/ldap/configure', config);
        return response.data;
    }

    /**
     * Get LDAP configuration
     * @returns LDAP configuration (password sanitized)
     */
    async getLdapConfig(): Promise<LdapConfig> {
        const response = await this.client.get<LdapConfig>('/api/v1/auth/ldap/config');
        return response.data;
    }

    /**
     * Test LDAP connection
     * @returns Connection test result
     */
    async testLdapConnection(): Promise<TestConnectionResult> {
        const response = await this.client.post<TestConnectionResult>('/api/v1/auth/ldap/test');
        return response.data;
    }

    /**
     * Configure OAuth provider
     * @param config OAuth provider configuration
     */
    async configureOAuth(config: OAuthProviderConfig): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>('/api/v1/auth/oauth/configure', config);
        return response.data;
    }

    /**
     * Get list of OAuth providers
     * @returns List of configured OAuth providers
     */
    async getOAuthProviders(): Promise<OAuthProviderList> {
        const response = await this.client.get<OAuthProviderList>('/api/v1/auth/oauth/providers');
        return response.data;
    }

    /**
     * Configure SSO (SAML)
     * @param config SSO configuration
     */
    async configureSso(config: SsoConfig): Promise<SuccessResponse> {
        const response = await this.client.post<SuccessResponse>('/api/v1/auth/sso/configure', config);
        return response.data;
    }

    /**
     * Get SAML metadata
     * @returns SAML metadata
     */
    async getSamlMetadata(): Promise<SamlMetadata> {
        const response = await this.client.get<SamlMetadata>('/api/v1/auth/sso/metadata');
        return response.data;
    }

    // ========================================================================
    // Utility Methods
    // ========================================================================

    /**
     * Set custom request headers
     * @param headers Headers to set
     */
    setHeaders(headers: Record<string, string>): void {
        Object.assign(this.client.defaults.headers, headers);
    }

    /**
     * Set authentication token
     * @param token Bearer token
     */
    setAuthToken(token: string): void {
        this.client.defaults.headers.Authorization = `Bearer ${token}`;
    }

    /**
     * Clear authentication token
     */
    clearAuthToken(): void {
        delete this.client.defaults.headers.Authorization;
    }

    /**
     * Get base URL
     */
    getBaseUrl(): string {
        return this.baseUrl;
    }

    /**
     * Set timeout for requests
     * @param timeout Timeout in milliseconds
     */
    setTimeout(timeout: number): void {
        this.client.defaults.timeout = timeout;
    }
}

// Default export
export default RustyDBSecurityClient;
