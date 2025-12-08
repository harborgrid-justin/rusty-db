# Security Vault Quick Start Guide

## Basic Usage Examples

### 1. Initialize Security Vault

```rust
use rusty_db::security_vault::SecurityVaultManager;

// Create vault manager
let mut vault = SecurityVaultManager::new("/var/lib/rustydb/security_vault".to_string())?;

// Create security context for a session
let context = vault.create_security_context(
    "user123".to_string(),
    "session456".to_string(),
    "192.168.1.100".to_string(),
);
```

### 2. Enable Transparent Data Encryption

```rust
// Enable tablespace-level encryption
vault.enable_tablespace_encryption("sensitive_data_ts", "AES256GCM").await?;

// Enable column-level encryption
vault.enable_column_encryption("customers", "credit_card", "CHACHA20").await?;

// Access TDE engine directly
let tde = vault.tde_engine();
let encrypted_data = tde.read().encrypt_tablespace_data(
    "sensitive_data_ts",
    plaintext_data
)?;
```

### 3. Configure Data Masking

```rust
// Create masking policies
vault.create_masking_policy("mask_ssn", ".*ssn.*|.*social.*", "SSN_MASK").await?;
vault.create_masking_policy("mask_email", ".*email.*", "EMAIL_MASK").await?;
vault.create_masking_policy("mask_cc", ".*credit.*card.*", "CREDIT_CARD_MASK").await?;

// Apply masking in queries
let masking = vault.masking_engine();
let masked_value = masking.read().mask_value(
    "customers",
    "ssn",
    "123-45-6789"
)?; // Returns: "***-**-6789"
```

### 4. Set Up Virtual Private Database (VPD)

```rust
// Create row-level security policy
vault.create_vpd_policy(
    "employees",
    "department_id = ${DEPT_ID} OR ${ROLE} = 'MANAGER'"
).await?;

// Create column-level security
let vpd = vault.vpd_engine();
vpd.write().create_column_policy(
    "hide_salary_policy".to_string(),
    "employees".to_string(),
    "salary".to_string(),
    ColumnAction::Nullify,
)?;

// Queries are automatically rewritten with security predicates
```

### 5. Audit Security Events

```rust
// Security events are automatically audited
// Manual audit logging:
let audit = vault.audit_vault();
audit.lock().await.log_security_event(
    "admin",
    "KEY_ROTATION",
    "Rotated encryption keys for tablespace: users_ts"
)?;

// Verify audit trail integrity
let is_valid = vault.verify_audit_integrity().await?;
assert!(is_valid);

// Generate compliance report
let report = vault.generate_compliance_report(
    "GDPR",
    start_timestamp,
    end_timestamp,
).await?;

println!("Total records: {}", report.total_records);
println!("Security events: {}", report.security_events);
```

### 6. Manage Privileges

```rust
let analyzer = vault.privilege_analyzer();
let mut priv_analyzer = analyzer.write();

// Grant privileges
priv_analyzer.grant_privilege(
    "user123",
    PrivilegeType::Object {
        privilege: "SELECT".to_string(),
        object_type: "TABLE".to_string(),
        object_name: "employees".to_string(),
    },
    "admin"
)?;

// Create roles
priv_analyzer.create_role("DATA_ANALYST")?;
priv_analyzer.grant_to_role(
    "DATA_ANALYST",
    PrivilegeType::System("CREATE VIEW".to_string())
)?;
priv_analyzer.grant_role("user123", "DATA_ANALYST")?;

// Analyze privileges
let recommendations = vault.analyze_user_privileges("user123")?;
for rec in recommendations {
    match rec {
        PrivilegeRecommendation::RevokeUnused { user_id, privilege, .. } => {
            println!("Recommend revoking unused privilege: {:?}", privilege);
        }
        _ => {}
    }
}
```

### 7. Key Rotation

```rust
// Rotate all expired keys
let rotated_count = vault.rotate_keys().await?;
println!("Rotated {} encryption keys", rotated_count);

// Manual key rotation
let keystore = vault.key_store();
let mut ks = keystore.lock().await;

// Rotate specific DEK
ks.rotate_dek("tablespace_users")?;

// Rotate MEK and re-encrypt all DEKs
ks.generate_mek()?;
ks.reencrypt_all_deks()?;
```

### 8. Get Security Statistics

```rust
// Encryption statistics
let enc_stats = vault.get_encryption_stats();
println!("Total encryptions: {}", enc_stats.encrypt_operations);
println!("Total decryptions: {}", enc_stats.decrypt_operations);
println!("Bytes encrypted: {}", enc_stats.bytes_encrypted);

// Audit statistics
let audit_stats = vault.get_audit_stats();
println!("Total audit records: {}", audit_stats.total_records);
println!("Tamper alerts: {}", audit_stats.tamper_alerts);
```

## Complete Workflow Example

```rust
use rusty_db::security_vault::*;

#[tokio::main]
async fn main() -> rusty_db::Result<()> {
    // 1. Initialize security vault
    let mut vault = SecurityVaultManager::new("/secure/vault".to_string())?;

    // 2. Set up encryption
    vault.enable_tablespace_encryption("customer_data", "AES256GCM").await?;
    vault.enable_column_encryption("customers", "ssn", "AES256GCM").await?;
    vault.enable_column_encryption("customers", "credit_card", "CHACHA20").await?;

    // 3. Configure data masking
    vault.create_masking_policy("mask_pii", "ssn|credit.*card", "PARTIAL_MASK").await?;

    // 4. Set up row-level security
    vault.create_vpd_policy(
        "customers",
        "region = ${USER_REGION} OR ${ROLE} = 'ADMIN'"
    ).await?;

    // 5. Create user session
    let context = vault.create_security_context(
        "analyst1".to_string(),
        "session_001".to_string(),
        "192.168.1.50".to_string(),
    );

    // 6. Set up privilege analysis
    let analyzer = vault.privilege_analyzer();
    let mut priv = analyzer.write();

    // Create roles
    priv.create_role("DATA_ANALYST")?;
    priv.grant_to_role(
        "DATA_ANALYST",
        PrivilegeType::Object {
            privilege: "SELECT".to_string(),
            object_type: "TABLE".to_string(),
            object_name: "customers".to_string(),
        }
    )?;

    // Assign role
    priv.grant_role("analyst1", "DATA_ANALYST")?;
    drop(priv);

    // 7. Periodic maintenance
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;

            // Daily key rotation check
            if let Ok(count) = vault.rotate_keys().await {
                println!("Rotated {} keys", count);
            }

            // Verify audit integrity
            if let Ok(valid) = vault.verify_audit_integrity().await {
                if !valid {
                    eprintln!("WARNING: Audit trail integrity compromised!");
                }
            }
        }
    });

    // 8. Generate monthly compliance report
    let end = chrono::Utc::now().timestamp();
    let start = end - (30 * 86400); // 30 days ago

    let report = vault.generate_compliance_report("GDPR", start, end).await?;
    println!("GDPR Compliance Report:");
    println!("  Total records: {}", report.total_records);
    println!("  Security events: {}", report.security_events);
    println!("  Failed operations: {}", report.failed_operations);

    Ok(())
}
```

## Configuration

```rust
use rusty_db::security_vault::VaultConfig;

let config = VaultConfig {
    data_dir: PathBuf::from("/var/lib/rustydb/vault"),
    default_tde_enabled: true,
    default_algorithm: "AES256GCM".to_string(),
    audit_enabled: true,
    audit_retention_days: 365,
    vpd_enabled: true,
    key_rotation_days: 90,
    hsm_enabled: false,
    hsm_config: None,
};

let vault = SecurityVaultManager::with_config(config)?;
```

## Security Best Practices

1. **Key Management**
   - Store MEK password in secure location (HSM, vault, etc.)
   - Rotate keys regularly (default: 90 days)
   - Never log or expose key material

2. **Audit Integrity**
   - Verify audit trail integrity daily
   - Monitor for tamper alerts
   - Archive audit logs to write-once storage

3. **Privilege Management**
   - Run privilege analysis monthly
   - Implement least privilege principle
   - Review and revoke unused privileges

4. **Data Masking**
   - Use consistent masking for analytics
   - Apply format-preserving encryption when needed
   - Test masked data doesn't reveal sensitive info

5. **VPD Policies**
   - Keep predicates simple for performance
   - Test policies thoroughly
   - Document all security policies

## Troubleshooting

### Encryption Performance Issues
```rust
// Check encryption statistics
let stats = vault.get_encryption_stats();
println!("Encryption ops: {}", stats.encrypt_operations);
println!("Failed ops: {}", stats.failed_operations);

// Consider using ChaCha20 for software-only environments
vault.enable_tablespace_encryption("data_ts", "CHACHA20").await?;
```

### Audit Trail Growing Too Large
```rust
let audit = vault.audit_vault();
let purged = audit.lock().await.purge_old_records()?;
println!("Purged {} old audit records", purged);
```

### VPD Policy Not Applied
```rust
let vpd = vault.vpd_engine();

// Check if policy exists
let policies = vpd.read().list_policies();
println!("Policies: {:?}", policies);

// Enable policy if disabled
vpd.write().enable_policy("my_policy")?;
```

## Performance Tips

1. **Use tablespace encryption for bulk data** (better performance than column-level)
2. **Batch masking operations** when possible
3. **Cache security contexts** at session level
4. **Pre-warm key cache** during startup
5. **Use async operations** for I/O-bound security checks
