/**
 * RustyDB Security Layer API Client Test Suite
 *
 * Comprehensive tests for all Security Layer REST endpoints including:
 * - Transparent Data Encryption (TDE)
 * - Data Masking Policies
 * - Virtual Private Database (VPD)
 * - Role-Based Access Control (RBAC)
 * - Audit Logging and Compliance
 * - Insider Threat Detection
 * - Enterprise Authentication (LDAP, OAuth, SSO)
 *
 * @module security.test
 */

import { describe, it, expect, beforeAll, afterAll } from '@jest/globals';
import RustyDBSecurityClient, {
    EnableEncryptionRequest,
    EnableColumnEncryptionRequest,
    KeyGenerationRequest,
    CreateMaskingPolicy,
    UpdateMaskingPolicy,
    MaskingTest,
    CreateVpdPolicy,
    UpdateVpdPolicy,
    TestVpdPredicate,
    CreateRoleRequest,
    UpdateRoleRequest,
    AssignPermissionsRequest,
    AuditQueryParams,
    AuditExportConfig,
    ComplianceParams,
    LdapConfig,
    OAuthProviderConfig,
    SsoConfig,
} from '../src/api/security';

describe('RustyDB Security Layer API Client', () => {
    let client: RustyDBSecurityClient;

    beforeAll(() => {
        // Initialize client with test configuration
        client = new RustyDBSecurityClient({
            baseUrl: process.env.RUSTYDB_URL || 'http://localhost:8080',
            timeout: 30000,
        });

        // Set auth token if available
        const token = process.env.RUSTYDB_TOKEN;
        if (token) {
            client.setAuthToken(token);
        }
    });

    afterAll(() => {
        client.clearAuthToken();
    });

    // ========================================================================
    // Encryption Tests
    // ========================================================================

    describe('Encryption API', () => {
        it('should get encryption status', async () => {
            const status = await client.getEncryptionStatus();
            expect(status).toBeDefined();
            expect(status).toHaveProperty('tde_enabled');
            expect(status).toHaveProperty('default_algorithm');
            expect(status).toHaveProperty('encrypted_tablespaces');
            expect(status).toHaveProperty('encrypted_columns');
            expect(status).toHaveProperty('total_bytes_encrypted');
            expect(status).toHaveProperty('total_bytes_decrypted');
            expect(status).toHaveProperty('key_rotation_status');
            expect(typeof status.tde_enabled).toBe('boolean');
            expect(Array.isArray(status.encrypted_tablespaces)).toBe(true);
            expect(Array.isArray(status.encrypted_columns)).toBe(true);
        });

        it('should enable TDE for a tablespace', async () => {
            const request: EnableEncryptionRequest = {
                tablespace_name: 'USERS',
                algorithm: 'AES256GCM',
                compress_before_encrypt: true,
            };

            const result = await client.enableEncryption(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('success');
            expect(result).toHaveProperty('message');
            expect(result).toHaveProperty('affected_objects');
            expect(result.success).toBe(true);
            expect(Array.isArray(result.affected_objects)).toBe(true);
        });

        it('should enable column-level encryption', async () => {
            const request: EnableColumnEncryptionRequest = {
                table_name: 'customers',
                column_name: 'credit_card',
                algorithm: 'AES256GCM',
            };

            const result = await client.enableColumnEncryption(request);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
            expect(result.message).toContain('credit_card');
        });

        it('should generate encryption key', async () => {
            const request: KeyGenerationRequest = {
                key_type: 'DEK',
                algorithm: 'AES-256-GCM',
                key_name: 'test_key_' + Date.now(),
            };

            const result = await client.generateKey(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('success');
            expect(result).toHaveProperty('key_id');
            expect(result).toHaveProperty('key_version');
            expect(result).toHaveProperty('algorithm');
            expect(result).toHaveProperty('created_at');
            expect(result.success).toBe(true);
            expect(result.key_version).toBeGreaterThan(0);
        });

        it('should list encryption keys', async () => {
            const keys = await client.listKeys();
            expect(Array.isArray(keys)).toBe(true);
            if (keys.length > 0) {
                const key = keys[0];
                expect(key).toHaveProperty('key_id');
                expect(key).toHaveProperty('key_version');
                expect(key).toHaveProperty('algorithm');
            }
        });

        it('should rotate encryption key', async () => {
            // First generate a key
            const genRequest: KeyGenerationRequest = {
                key_type: 'DEK',
                algorithm: 'AES-256-GCM',
                key_name: 'rotate_test_key_' + Date.now(),
            };
            const genResult = await client.generateKey(genRequest);

            // Then rotate it
            const rotateResult = await client.rotateKey(genResult.key_id);
            expect(rotateResult).toBeDefined();
            expect(rotateResult.success).toBe(true);
            expect(rotateResult.key_id).toBe(genResult.key_id);
            expect(rotateResult.key_version).toBeGreaterThan(genResult.key_version);
        });
    });

    // ========================================================================
    // Data Masking Tests
    // ========================================================================

    describe('Data Masking API', () => {
        const testPolicyName = 'test_masking_policy_' + Date.now();

        it('should create masking policy', async () => {
            const request: CreateMaskingPolicy = {
                name: testPolicyName,
                column_pattern: 'ssn',
                table_pattern: 'employees',
                masking_type: 'PartialMask',
                priority: 100,
            };

            const result = await client.createMaskingPolicy(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('name');
            expect(result).toHaveProperty('column_pattern');
            expect(result).toHaveProperty('masking_type');
            expect(result).toHaveProperty('enabled');
            expect(result).toHaveProperty('priority');
            expect(result).toHaveProperty('created_at');
            expect(result.name).toBe(testPolicyName);
        });

        it('should list masking policies', async () => {
            const policies = await client.listMaskingPolicies();
            expect(Array.isArray(policies)).toBe(true);
        });

        it('should get masking policy by name', async () => {
            const policy = await client.getMaskingPolicy(testPolicyName);
            expect(policy).toBeDefined();
            expect(policy.name).toBe(testPolicyName);
        });

        it('should update masking policy', async () => {
            const request: UpdateMaskingPolicy = {
                enabled: false,
                priority: 200,
            };

            const result = await client.updateMaskingPolicy(testPolicyName, request);
            expect(result).toBeDefined();
            expect(result.name).toBe(testPolicyName);
        });

        it('should test masking policy', async () => {
            const request: MaskingTest = {
                policy_name: testPolicyName,
                test_values: ['123-45-6789', '987-65-4321', '555-12-3456'],
            };

            const result = await client.testMasking(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('policy_name');
            expect(result).toHaveProperty('results');
            expect(result.policy_name).toBe(testPolicyName);
            expect(Array.isArray(result.results)).toBe(true);
            expect(result.results.length).toBe(3);

            for (const testCase of result.results) {
                expect(testCase).toHaveProperty('original');
                expect(testCase).toHaveProperty('masked');
                expect(testCase).toHaveProperty('masking_type');
            }
        });

        it('should disable masking policy', async () => {
            const result = await client.disableMaskingPolicy(testPolicyName);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });

        it('should enable masking policy', async () => {
            const result = await client.enableMaskingPolicy(testPolicyName);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });

        it('should delete masking policy', async () => {
            const result = await client.deleteMaskingPolicy(testPolicyName);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });
    });

    // ========================================================================
    // VPD Tests
    // ========================================================================

    describe('Virtual Private Database (VPD) API', () => {
        const testPolicyName = 'test_vpd_policy_' + Date.now();

        it('should create VPD policy', async () => {
            const request: CreateVpdPolicy = {
                name: testPolicyName,
                table_name: 'employees',
                schema_name: 'hr',
                predicate: "department_id = SYS_CONTEXT('USERENV', 'DEPARTMENT_ID')",
                policy_scope: 'SELECT',
            };

            const result = await client.createVpdPolicy(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('name');
            expect(result).toHaveProperty('table_name');
            expect(result).toHaveProperty('predicate');
            expect(result).toHaveProperty('policy_scope');
            expect(result).toHaveProperty('enabled');
            expect(result).toHaveProperty('created_at');
            expect(result.name).toBe(testPolicyName);
        });

        it('should list VPD policies', async () => {
            const policies = await client.listVpdPolicies();
            expect(Array.isArray(policies)).toBe(true);
        });

        it('should get VPD policy by name', async () => {
            const policy = await client.getVpdPolicy(testPolicyName);
            expect(policy).toBeDefined();
            expect(policy.name).toBe(testPolicyName);
        });

        it('should get policies for a table', async () => {
            const policies = await client.getTablePolicies('employees');
            expect(Array.isArray(policies)).toBe(true);
        });

        it('should test VPD predicate', async () => {
            const request: TestVpdPredicate = {
                predicate: "department_id = {dept_id}",
                context: {
                    dept_id: '100',
                },
            };

            const result = await client.testVpdPredicate(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('original_predicate');
            expect(result).toHaveProperty('evaluated_predicate');
            expect(result).toHaveProperty('context_used');
            expect(result).toHaveProperty('valid_sql');
            expect(typeof result.valid_sql).toBe('boolean');
        });

        it('should update VPD policy', async () => {
            const request: UpdateVpdPolicy = {
                enabled: false,
                predicate: "department_id = {dept_id}",
            };

            const result = await client.updateVpdPolicy(testPolicyName, request);
            expect(result).toBeDefined();
            expect(result.name).toBe(testPolicyName);
        });

        it('should disable VPD policy', async () => {
            const result = await client.disableVpdPolicy(testPolicyName);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });

        it('should enable VPD policy', async () => {
            const result = await client.enableVpdPolicy(testPolicyName);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });

        it('should delete VPD policy', async () => {
            const result = await client.deleteVpdPolicy(testPolicyName);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });
    });

    // ========================================================================
    // RBAC Tests
    // ========================================================================

    describe('Role-Based Access Control (RBAC) API', () => {
        const testRoleId = 'test_role_' + Date.now();

        it('should create role', async () => {
            const request: CreateRoleRequest = {
                id: testRoleId,
                name: 'Test Role',
                description: 'A test role for automated testing',
                parent_roles: [],
                permissions: ['select', 'insert'],
            };

            const result = await client.createRole(request);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('id');
            expect(result).toHaveProperty('name');
            expect(result).toHaveProperty('permissions');
            expect(result).toHaveProperty('is_active');
            expect(result).toHaveProperty('created_at');
            expect(result).toHaveProperty('updated_at');
            expect(result.id).toBe(testRoleId);
            expect(result.is_active).toBe(true);
        });

        it('should list roles', async () => {
            const roles = await client.listRoles();
            expect(Array.isArray(roles)).toBe(true);
        });

        it('should get role by ID', async () => {
            const role = await client.getRole(testRoleId);
            expect(role).toBeDefined();
            expect(role.id).toBe(testRoleId);
        });

        it('should list permissions', async () => {
            const permissions = await client.listPermissions();
            expect(Array.isArray(permissions)).toBe(true);
            if (permissions.length > 0) {
                const perm = permissions[0];
                expect(perm).toHaveProperty('id');
                expect(perm).toHaveProperty('name');
                expect(perm).toHaveProperty('description');
                expect(perm).toHaveProperty('category');
            }
        });

        it('should assign permissions to role', async () => {
            const request: AssignPermissionsRequest = {
                permissions: ['update', 'delete'],
            };

            const result = await client.assignPermissions(testRoleId, request);
            expect(result).toBeDefined();
            expect(result.id).toBe(testRoleId);
            expect(result.permissions.length).toBeGreaterThan(0);
        });

        it('should update role', async () => {
            const request: UpdateRoleRequest = {
                description: 'Updated description',
                is_active: false,
            };

            const result = await client.updateRole(testRoleId, request);
            expect(result).toBeDefined();
            expect(result.id).toBe(testRoleId);
        });

        it('should delete role', async () => {
            const result = await client.deleteRole(testRoleId);
            expect(result).toBeDefined();
            expect(result.success).toBe(true);
        });
    });

    // ========================================================================
    // Threat Detection Tests
    // ========================================================================

    describe('Threat Detection API', () => {
        it('should get threat status', async () => {
            const status = await client.getThreatStatus();
            expect(status).toBeDefined();
            expect(status).toHaveProperty('enabled');
            expect(status).toHaveProperty('total_assessments');
            expect(status).toHaveProperty('critical_threats');
            expect(status).toHaveProperty('high_threats');
            expect(status).toHaveProperty('blocked_queries');
            expect(status).toHaveProperty('exfiltration_attempts');
            expect(status).toHaveProperty('escalation_attempts');
            expect(status).toHaveProperty('baselines_established');
            expect(typeof status.enabled).toBe('boolean');
            expect(typeof status.total_assessments).toBe('number');
        });

        it('should get threat history', async () => {
            const history = await client.getThreatHistory();
            expect(Array.isArray(history)).toBe(true);
            // History might be empty in test environment
        });

        it('should get insider threat status', async () => {
            const status = await client.getInsiderThreatStatus();
            expect(status).toBeDefined();
            expect(status).toHaveProperty('enabled');
            expect(status).toHaveProperty('auto_block_critical');
            expect(status).toHaveProperty('require_mfa_high_risk');
            expect(status).toHaveProperty('alert_threshold');
            expect(status).toHaveProperty('block_threshold');
            expect(status).toHaveProperty('behavioral_analytics_enabled');
            expect(status).toHaveProperty('anomaly_detection_enabled');
            expect(status).toHaveProperty('exfiltration_prevention_enabled');
            expect(status).toHaveProperty('escalation_detection_enabled');
            expect(typeof status.enabled).toBe('boolean');
        });
    });

    // ========================================================================
    // Audit Tests
    // ========================================================================

    describe('Audit API', () => {
        it('should query audit logs', async () => {
            const params: AuditQueryParams = {
                limit: 10,
                offset: 0,
            };

            const logs = await client.queryAuditLogs(params);
            expect(Array.isArray(logs)).toBe(true);
        });

        it('should query audit logs with filters', async () => {
            const now = Date.now();
            const params: AuditQueryParams = {
                start_time: Math.floor(now / 1000) - 86400, // Last 24 hours
                end_time: Math.floor(now / 1000),
                limit: 100,
            };

            const logs = await client.queryAuditLogs(params);
            expect(Array.isArray(logs)).toBe(true);
        });

        it('should get audit stats', async () => {
            const stats = await client.getAuditStats();
            expect(stats).toBeDefined();
            expect(stats).toHaveProperty('total_records');
        });

        it('should export audit logs', async () => {
            const now = Date.now();
            const config: AuditExportConfig = {
                format: 'json',
                start_time: Math.floor(now / 1000) - 86400,
                end_time: Math.floor(now / 1000),
                destination: '/tmp/audit_export',
                include_sensitive: false,
            };

            const result = await client.exportAuditLogs(config);
            expect(result).toBeDefined();
            expect(result).toHaveProperty('success');
            expect(result).toHaveProperty('records_exported');
            expect(result).toHaveProperty('file_path');
            expect(result).toHaveProperty('file_size_bytes');
            expect(result).toHaveProperty('checksum');
        });

        it('should generate compliance report', async () => {
            const now = Date.now();
            const params: ComplianceParams = {
                regulation: 'GDPR',
                start_date: Math.floor(now / 1000) - 2592000, // Last 30 days
                end_date: Math.floor(now / 1000),
                include_recommendations: true,
            };

            const report = await client.complianceReport(params);
            expect(report).toBeDefined();
            expect(report).toHaveProperty('regulation');
            expect(report).toHaveProperty('period_start');
            expect(report).toHaveProperty('period_end');
            expect(report).toHaveProperty('compliant');
            expect(report).toHaveProperty('total_audit_records');
            expect(report).toHaveProperty('violations');
            expect(report).toHaveProperty('recommendations');
            expect(report).toHaveProperty('generated_at');
            expect(report.regulation).toBe('GDPR');
            expect(Array.isArray(report.violations)).toBe(true);
            expect(Array.isArray(report.recommendations)).toBe(true);
        });

        it('should verify audit integrity', async () => {
            const result = await client.verifyAuditIntegrity();
            expect(result).toBeDefined();
            expect(result).toHaveProperty('valid');
            expect(result).toHaveProperty('verified_at');
            expect(result).toHaveProperty('message');
            expect(typeof result.valid).toBe('boolean');
        });
    });

    // ========================================================================
    // Network Security Tests
    // ========================================================================

    describe('Network Security API', () => {
        it('should get circuit breaker status', async () => {
            const breakers = await client.getCircuitBreakers();
            expect(Array.isArray(breakers)).toBe(true);
            if (breakers.length > 0) {
                const breaker = breakers[0];
                expect(breaker).toHaveProperty('name');
                expect(breaker).toHaveProperty('state');
                expect(breaker).toHaveProperty('failure_count');
                expect(breaker).toHaveProperty('success_count');
                expect(breaker).toHaveProperty('failure_threshold');
                expect(breaker).toHaveProperty('timeout_secs');
            }
        });
    });

    // ========================================================================
    // Enterprise Authentication Tests
    // ========================================================================

    describe('Enterprise Authentication API', () => {
        describe('LDAP', () => {
            it('should configure LDAP', async () => {
                const config: LdapConfig = {
                    enabled: false, // Disable for testing
                    server_url: 'ldap://test.example.com:389',
                    bind_dn: 'cn=admin,dc=example,dc=com',
                    bind_password: 'test_password',
                    base_dn: 'dc=example,dc=com',
                    user_filter: '(&(objectClass=person)(uid={username}))',
                    group_filter: '(&(objectClass=groupOfNames)(member={dn}))',
                    use_tls: true,
                    verify_certificate: true,
                    timeout_secs: 30,
                };

                const result = await client.configureLdap(config);
                expect(result).toBeDefined();
                expect(result.success).toBe(true);
            });

            it('should get LDAP config', async () => {
                const config = await client.getLdapConfig();
                expect(config).toBeDefined();
                expect(config).toHaveProperty('enabled');
                expect(config).toHaveProperty('server_url');
                expect(config).toHaveProperty('base_dn');
                expect(config).toHaveProperty('use_tls');
                // Password should be sanitized
                if (config.bind_password) {
                    expect(config.bind_password).toBe('********');
                }
            });

            it('should test LDAP connection', async () => {
                const result = await client.testLdapConnection();
                expect(result).toBeDefined();
                expect(result).toHaveProperty('success');
                expect(result).toHaveProperty('message');
                expect(result).toHaveProperty('response_time_ms');
                expect(typeof result.success).toBe('boolean');
            });
        });

        describe('OAuth', () => {
            it('should configure OAuth provider', async () => {
                const config: OAuthProviderConfig = {
                    provider_name: 'Test OAuth Provider',
                    provider_type: 'custom',
                    client_id: 'test_client_id',
                    client_secret: 'test_client_secret',
                    authorization_endpoint: 'https://oauth.example.com/authorize',
                    token_endpoint: 'https://oauth.example.com/token',
                    user_info_endpoint: 'https://oauth.example.com/userinfo',
                    scopes: ['openid', 'profile', 'email'],
                    enabled: false,
                };

                const result = await client.configureOAuth(config);
                expect(result).toBeDefined();
                expect(result.success).toBe(true);
            });

            it('should list OAuth providers', async () => {
                const result = await client.getOAuthProviders();
                expect(result).toBeDefined();
                expect(result).toHaveProperty('providers');
                expect(result).toHaveProperty('total_count');
                expect(Array.isArray(result.providers)).toBe(true);
            });
        });

        describe('SSO (SAML)', () => {
            it('should configure SSO', async () => {
                const config: SsoConfig = {
                    enabled: false,
                    entity_id: 'https://rustydb.test.local/saml/metadata',
                    sso_url: 'https://rustydb.test.local/saml/sso',
                    slo_url: 'https://rustydb.test.local/saml/slo',
                    certificate: 'test_cert',
                    private_key: 'test_key',
                    idp_entity_id: 'https://idp.example.com',
                    idp_sso_url: 'https://idp.example.com/sso',
                    idp_certificate: 'idp_cert',
                    name_id_format: 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',
                    attributes_mapping: {
                        email: 'mail',
                        firstName: 'givenName',
                        lastName: 'sn',
                    },
                };

                const result = await client.configureSso(config);
                expect(result).toBeDefined();
                expect(result.success).toBe(true);
            });

            it('should get SAML metadata', async () => {
                const metadata = await client.getSamlMetadata();
                expect(metadata).toBeDefined();
                expect(metadata).toHaveProperty('entity_id');
                expect(metadata).toHaveProperty('sso_url');
                expect(metadata).toHaveProperty('certificate');
                expect(metadata).toHaveProperty('metadata_xml');
                expect(metadata.metadata_xml).toContain('EntityDescriptor');
            });
        });
    });

    // ========================================================================
    // Utility Tests
    // ========================================================================

    describe('Utility Methods', () => {
        it('should set custom headers', () => {
            client.setHeaders({
                'X-Custom-Header': 'test-value',
            });
            expect(client.getBaseUrl()).toBe(process.env.RUSTYDB_URL || 'http://localhost:8080');
        });

        it('should set and clear auth token', () => {
            client.setAuthToken('test-token');
            client.clearAuthToken();
            expect(true).toBe(true); // Just verify no errors
        });

        it('should set timeout', () => {
            client.setTimeout(60000);
            expect(true).toBe(true); // Just verify no errors
        });

        it('should get base URL', () => {
            const baseUrl = client.getBaseUrl();
            expect(typeof baseUrl).toBe('string');
            expect(baseUrl.startsWith('http')).toBe(true);
        });
    });
});

// ============================================================================
// Integration Test Examples
// ============================================================================

describe('Security Integration Tests', () => {
    let client: RustyDBSecurityClient;

    beforeAll(() => {
        client = new RustyDBSecurityClient({
            baseUrl: process.env.RUSTYDB_URL || 'http://localhost:8080',
        });
    });

    it('should complete full encryption workflow', async () => {
        // 1. Get initial status
        const initialStatus = await client.getEncryptionStatus();
        expect(initialStatus).toBeDefined();

        // 2. Generate a key
        const keyRequest: KeyGenerationRequest = {
            key_type: 'DEK',
            algorithm: 'AES-256-GCM',
            key_name: 'integration_test_key_' + Date.now(),
        };
        const key = await client.generateKey(keyRequest);
        expect(key.success).toBe(true);

        // 3. Enable TDE
        const tdeRequest: EnableEncryptionRequest = {
            tablespace_name: 'TEST_TABLESPACE',
            algorithm: 'AES256GCM',
            compress_before_encrypt: true,
        };
        const tdeResult = await client.enableEncryption(tdeRequest);
        expect(tdeResult.success).toBe(true);

        // 4. List keys
        const keys = await client.listKeys();
        expect(keys.length).toBeGreaterThan(0);
    });

    it('should complete full masking workflow', async () => {
        const policyName = 'integration_mask_' + Date.now();

        // 1. Create policy
        const createRequest: CreateMaskingPolicy = {
            name: policyName,
            column_pattern: 'ssn',
            masking_type: 'PartialMask',
            priority: 100,
        };
        const created = await client.createMaskingPolicy(createRequest);
        expect(created.name).toBe(policyName);

        // 2. Test masking
        const testRequest: MaskingTest = {
            policy_name: policyName,
            test_values: ['123-45-6789'],
        };
        const testResult = await client.testMasking(testRequest);
        expect(testResult.results.length).toBe(1);

        // 3. Disable policy
        await client.disableMaskingPolicy(policyName);

        // 4. Clean up
        await client.deleteMaskingPolicy(policyName);
    });

    it('should complete full audit workflow', async () => {
        // 1. Query recent logs
        const logs = await client.queryAuditLogs({ limit: 10 });
        expect(Array.isArray(logs)).toBe(true);

        // 2. Get stats
        const stats = await client.getAuditStats();
        expect(stats).toBeDefined();

        // 3. Verify integrity
        const integrity = await client.verifyAuditIntegrity();
        expect(integrity.valid).toBeDefined();

        // 4. Generate compliance report
        const now = Date.now();
        const report = await client.complianceReport({
            regulation: 'SOX',
            start_date: Math.floor(now / 1000) - 2592000,
            end_date: Math.floor(now / 1000),
        });
        expect(report.regulation).toBe('SOX');
    });
});

// Export test suite
export default describe;
