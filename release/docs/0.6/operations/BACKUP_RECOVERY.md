# RustyDB v0.6.0 - Backup and Recovery Guide

**Document Version**: 1.0
**Release**: v0.6.0
**Last Updated**: 2025-12-28
**Classification**: Enterprise Operations

---

## Table of Contents

1. [Backup Strategy](#backup-strategy)
2. [Backup Types](#backup-types)
3. [Backup Operations](#backup-operations)
4. [Recovery Operations](#recovery-operations)
5. [Point-in-Time Recovery](#point-in-time-recovery)
6. [Backup Automation](#backup-automation)
7. [Backup Verification](#backup-verification)
8. [Disaster Recovery](#disaster-recovery)
9. [Backup Best Practices](#backup-best-practices)

---

## Backup Strategy

### 3-2-1 Backup Rule

- **3** copies of data (production + 2 backups)
- **2** different storage media
- **1** offsite copy

### Backup Schedule Recommendation

| Backup Type | Frequency | Retention | Use Case |
|-------------|-----------|-----------|----------|
| Full | Weekly (Sunday) | 30 days | Complete recovery |
| Incremental | Daily | 7 days | Daily recovery |
| Differential | Optional | 7 days | Alternative to incremental |
| PITR Base | Weekly | 30 days | Point-in-time recovery |
| WAL Archive | Continuous | Until next full | PITR support |

### Recovery Objectives

**RTO (Recovery Time Objective)**:
- Full backup restore: 1-4 hours (depending on size)
- PITR: 30 minutes - 2 hours
- Table-level restore: 15-30 minutes

**RPO (Recovery Point Objective)**:
- With WAL archiving: < 1 minute data loss
- Without WAL: Last backup time
- With replication: < 10 seconds data loss

---

## Backup Types

### 1. Full Backup

**Description**: Complete copy of all database files

**Advantages**:
- Complete standalone backup
- Fastest restore
- No dependencies

**Disadvantages**:
- Largest size
- Longest backup time

**Use Cases**:
- Weekly baseline backup
- Before major upgrades
- Long-term archival

### 2. Incremental Backup

**Description**: Only changes since last backup (full or incremental)

**Advantages**:
- Smallest size
- Fastest backup
- Minimal storage

**Disadvantages**:
- Requires full backup chain
- Slower restore (multiple files)

**Use Cases**:
- Daily backups
- High-frequency backups
- Storage-constrained environments

### 3. Differential Backup

**Description**: Changes since last full backup

**Advantages**:
- Faster restore than incremental
- Only needs full + last differential

**Disadvantages**:
- Grows larger over time
- More storage than incremental

**Use Cases**:
- Alternative daily backup
- Balanced restore speed/storage

### 4. PITR (Point-in-Time Recovery) Backup

**Description**: Full backup + continuous WAL archiving

**Advantages**:
- Restore to any point in time
- Minimal data loss (< 1 minute)
- Excellent for compliance

**Disadvantages**:
- Requires WAL archiving setup
- More complex restore

**Use Cases**:
- Production databases
- Compliance requirements
- Mission-critical data

---

## Backup Operations

### Full Backup

**Command**:
```bash
rusty-db-backup --type full \
  --output /backups/rustydb/full_$(date +%Y%m%d_%H%M%S).backup \
  --compress \
  --encrypt \
  --verify
```

**With Parallel Threads** (faster for large databases):
```bash
rusty-db-backup --type full \
  --output /backups/rustydb/full_$(date +%Y%m%d_%H%M%S).backup \
  --threads 4 \
  --compress \
  --encrypt \
  --verify
```

**Specific Database**:
```bash
rusty-db-backup --type full \
  --database production_db \
  --output /backups/rustydb/prod_full_$(date +%Y%m%d_%H%M%S).backup
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -H "Content-Type: application/json" \
  -d '{
    "backup_type": "full",
    "compression": true,
    "encryption": true
  }'

# Response
{
  "backup_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "in_progress",
  "started_at": 1703721600,
  "location": "/backups/550e8400-e29b-41d4-a716-446655440000"
}
```

### Incremental Backup

**Command**:
```bash
rusty-db-backup --type incremental \
  --output /backups/rustydb/incr_$(date +%Y%m%d_%H%M%S).backup \
  --compress \
  --encrypt
```

**With Base Backup Reference**:
```bash
rusty-db-backup --type incremental \
  --base-backup /backups/rustydb/full_20251228_000000.backup \
  --output /backups/rustydb/incr_$(date +%Y%m%d_%H%M%S).backup
```

### Differential Backup

**Command**:
```bash
rusty-db-backup --type differential \
  --base-backup /backups/rustydb/full_20251228_000000.backup \
  --output /backups/rustydb/diff_$(date +%Y%m%d_%H%M%S).backup \
  --compress
```

### PITR Base Backup

**Step 1: Enable WAL Archiving** (`conf/rustydb.toml`):
```toml
[wal]
enabled = true
archive_enabled = true
archive_command = "cp %p /var/lib/rustydb/archive/%f"
```

**Step 2: Create Base Backup**:
```bash
rusty-db-backup --type pitr-base \
  --output /backups/rustydb/pitr_base_$(date +%Y%m%d_%H%M%S).backup \
  --compress \
  --encrypt
```

**Step 3: Archive WAL** (automatic once configured)

**Alternative Archive Commands**:
```bash
# Local archive
archive_command = "cp %p /var/lib/rustydb/archive/%f"

# S3 archive
archive_command = "aws s3 cp %p s3://rustydb-backup/wal/%f"

# Network archive with verification
archive_command = "rsync -a %p backup-server:/backups/wal/%f && test -f backup-server:/backups/wal/%f"
```

---

## Recovery Operations

### Full Restore

**Step 1: Stop Database** (if running):
```bash
sudo systemctl stop rustydb
```

**Step 2: Restore Backup**:
```bash
rusty-db-restore --input /backups/rustydb/full_20251228_000000.backup \
  --data-dir /var/lib/rustydb/instances/default/data \
  --threads 4
```

**Step 3: Start Database**:
```bash
sudo systemctl start rustydb
```

**Step 4: Verify**:
```bash
rusty-db-cli --command "SELECT * FROM v$database_health;"
rusty-db-cli --command "SELECT COUNT(*) FROM pg_tables;"
```

### Incremental Restore

**Step 1: Stop Database**:
```bash
sudo systemctl stop rustydb
```

**Step 2: Restore Full Backup**:
```bash
rusty-db-restore --input /backups/rustydb/full_20251228_000000.backup \
  --data-dir /var/lib/rustydb/instances/default/data
```

**Step 3: Apply Incremental Backups** (in order):
```bash
rusty-db-restore --input /backups/rustydb/incr_20251229_000000.backup \
  --data-dir /var/lib/rustydb/instances/default/data \
  --incremental

rusty-db-restore --input /backups/rustydb/incr_20251230_000000.backup \
  --data-dir /var/lib/rustydb/instances/default/data \
  --incremental
```

**Step 4: Start Database**:
```bash
sudo systemctl start rustydb
```

### Table-Level Restore

**Step 1: Restore to Temporary Location**:
```bash
rusty-db-restore --input /backups/rustydb/full_20251228_000000.backup \
  --table customers \
  --data-dir /var/lib/rustydb/temp \
  --no-start
```

**Step 2: Export Restored Table**:
```bash
rusty-db-export --table customers \
  --input-dir /var/lib/rustydb/temp \
  --output customers_restored.sql
```

**Step 3: Import to Production**:
```bash
# Option 1: Replace table
rusty-db-cli --command "DROP TABLE customers;"
rusty-db-cli < customers_restored.sql

# Option 2: Restore with different name
rusty-db-cli --command "CREATE TABLE customers_restored AS SELECT * FROM customers;"
rusty-db-cli < customers_restored.sql
```

---

## Point-in-Time Recovery

### PITR to Specific Timestamp

**Scenario**: Restore database to 2025-12-28 14:30:00

**Step 1: Stop Database**:
```bash
sudo systemctl stop rustydb
```

**Step 2: Restore Base Backup**:
```bash
rusty-db-restore --input /backups/rustydb/pitr_base_20251228_000000.backup \
  --data-dir /var/lib/rustydb/instances/default/data
```

**Step 3: Configure Recovery Target**:
```bash
cat > /var/lib/rustydb/instances/default/data/recovery.conf << EOF
recovery_target_time = '2025-12-28 14:30:00'
recovery_target_action = 'promote'
restore_command = 'cp /var/lib/rustydb/archive/%f %p'
EOF
```

**Step 4: Start Database** (enters recovery mode):
```bash
sudo systemctl start rustydb
```

**Step 5: Monitor Recovery**:
```bash
# Watch logs
sudo journalctl -u rustydb -f

# Check recovery status
rusty-db-cli --command "SELECT pg_is_in_recovery();"
```

**Step 6: Verify Target Time**:
```bash
rusty-db-cli --command "SELECT NOW();"  # Should be near target time
```

### PITR to Specific Transaction

**Restore to Transaction ID**:
```bash
cat > /var/lib/rustydb/instances/default/data/recovery.conf << EOF
recovery_target_xid = '1234567'
recovery_target_action = 'promote'
restore_command = 'cp /var/lib/rustydb/archive/%f %p'
EOF
```

### PITR Recovery Scenarios

**1. Accidental DELETE**:
- Identify transaction time of DELETE
- PITR to just before DELETE
- Verify data is restored
- Promote to normal operation

**2. Corrupted Data**:
- Identify when corruption started
- PITR to last known good state
- Investigate root cause
- Apply fix before promoting

**3. Failed Application Deployment**:
- PITR to before deployment
- Review deployment scripts
- Fix issues
- Redeploy correctly

---

## Backup Automation

### Automated Backup Script

**/usr/local/bin/rustydb-backup.sh**:
```bash
#!/bin/bash
# RustyDB Automated Backup Script

# Configuration
BACKUP_DIR="/backups/rustydb"
RETENTION_DAYS=30
DATE=$(date +%Y%m%d_%H%M%S)
DAY_OF_WEEK=$(date +%u)  # 1-7 (Monday-Sunday)
LOG_FILE="/var/log/rustydb/backup.log"

# Email configuration
ALERT_EMAIL="dba-team@company.com"
SMTP_SERVER="smtp.company.com"

# Function to log messages
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a $LOG_FILE
}

# Function to send alert email
send_alert() {
    local subject="$1"
    local message="$2"
    echo "$message" | mail -s "$subject" -S smtp=$SMTP_SERVER $ALERT_EMAIL
}

# Create backup directory
mkdir -p $BACKUP_DIR

log "Starting backup process"

# Determine backup type based on day of week
if [ "$DAY_OF_WEEK" -eq 7 ]; then
    # Sunday: Full backup
    log "Performing full backup"
    BACKUP_TYPE="full"
    BACKUP_FILE="$BACKUP_DIR/full_${DATE}.backup"

    rusty-db-backup --type full \
      --output $BACKUP_FILE \
      --compress \
      --encrypt \
      --verify \
      --threads 4 >> $LOG_FILE 2>&1

    BACKUP_STATUS=$?
else
    # Other days: Incremental backup
    log "Performing incremental backup"
    BACKUP_TYPE="incremental"
    BACKUP_FILE="$BACKUP_DIR/incr_${DATE}.backup"

    rusty-db-backup --type incremental \
      --output $BACKUP_FILE \
      --compress \
      --encrypt \
      --verify >> $LOG_FILE 2>&1

    BACKUP_STATUS=$?
fi

# Check backup status
if [ $BACKUP_STATUS -eq 0 ]; then
    log "Backup completed successfully: $BACKUP_FILE"
    BACKUP_SIZE=$(du -h $BACKUP_FILE | cut -f1)
    log "Backup size: $BACKUP_SIZE"

    # Verify backup
    log "Verifying backup integrity"
    rusty-db-backup --verify $BACKUP_FILE >> $LOG_FILE 2>&1
    if [ $? -eq 0 ]; then
        log "Backup verification successful"
    else
        log "ERROR: Backup verification failed"
        send_alert "RustyDB Backup Verification Failed" "Backup file: $BACKUP_FILE"
    fi

    # Upload to S3 (optional)
    if command -v aws &> /dev/null; then
        log "Uploading backup to S3"
        aws s3 cp $BACKUP_FILE s3://company-rustydb-backups/$(basename $BACKUP_FILE) \
            --storage-class GLACIER_IR >> $LOG_FILE 2>&1
        if [ $? -eq 0 ]; then
            log "S3 upload successful"
        else
            log "WARNING: S3 upload failed"
            send_alert "RustyDB Backup S3 Upload Failed" "Backup file: $BACKUP_FILE"
        fi
    fi

    # Clean up old backups
    log "Cleaning up old backups (retention: $RETENTION_DAYS days)"
    find $BACKUP_DIR -name "*.backup" -mtime +$RETENTION_DAYS -delete
    log "Old backups cleaned up"

else
    log "ERROR: Backup failed with status $BACKUP_STATUS"
    send_alert "RustyDB Backup Failed" "Backup type: $BACKUP_TYPE\nStatus: $BACKUP_STATUS\nCheck log: $LOG_FILE"
    exit 1
fi

log "Backup process completed"
exit 0
```

**Make Executable**:
```bash
sudo chmod +x /usr/local/bin/rustydb-backup.sh
```

**Schedule with Cron**:
```bash
# Run daily at 2 AM
sudo crontab -e

# Add line:
0 2 * * * /usr/local/bin/rustydb-backup.sh
```

**Schedule with systemd Timer**:

**/etc/systemd/system/rustydb-backup.service**:
```ini
[Unit]
Description=RustyDB Backup Service
After=rustydb.service

[Service]
Type=oneshot
ExecStart=/usr/local/bin/rustydb-backup.sh
User=rustydb
StandardOutput=journal
StandardError=journal
```

**/etc/systemd/system/rustydb-backup.timer**:
```ini
[Unit]
Description=RustyDB Backup Timer

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
```

**Enable Timer**:
```bash
sudo systemctl daemon-reload
sudo systemctl enable rustydb-backup.timer
sudo systemctl start rustydb-backup.timer

# Check timer status
sudo systemctl list-timers rustydb-backup.timer
```

---

## Backup Verification

### Verify Backup Integrity

**Command**:
```bash
rusty-db-backup --verify /backups/rustydb/full_20251228_000000.backup
```

**Output**:
```
Verifying backup: /backups/rustydb/full_20251228_000000.backup
✓ Backup file exists and is readable
✓ Backup metadata is valid
✓ Checksum verification passed
✓ Compression integrity verified
✓ Encryption integrity verified
✓ Data file consistency checked
Verification completed successfully
```

### List Backup Contents

**Command**:
```bash
rusty-db-backup --list /backups/rustydb/full_20251228_000000.backup
```

**Output**:
```
Backup Information:
  Type: Full
  Date: 2025-12-28 00:00:00
  Size: 2.5 GB (compressed)
  Compression: LZ4
  Encryption: AES-256
  Database Version: 0.6.0

Contents:
  - Database: production_db (1.8 GB)
    - Table: customers (500 MB)
    - Table: orders (800 MB)
    - Table: products (300 MB)
    - Indexes: (200 MB)
  - System catalogs (50 MB)
  - Configuration (1 MB)
```

### Check Backup Metadata

**Command**:
```bash
rusty-db-backup --info /backups/rustydb/full_20251228_000000.backup
```

**Output**:
```json
{
  "backup_id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "full",
  "created_at": "2025-12-28T00:00:00Z",
  "size_bytes": 2684354560,
  "compressed_size_bytes": 1073741824,
  "compression_algorithm": "lz4",
  "encryption_enabled": true,
  "database_version": "0.6.0",
  "checksum": "sha256:a3b2c1d4e5f6...",
  "databases": ["production_db"],
  "verified": true,
  "verified_at": "2025-12-28T00:05:00Z"
}
```

### Test Restore (Dry Run)

**Command**:
```bash
rusty-db-restore --input /backups/rustydb/full_20251228_000000.backup \
  --data-dir /tmp/restore-test \
  --dry-run
```

---

## Disaster Recovery

### DR Procedures

**Complete Disaster Recovery Workflow**:

**1. Assess Situation**:
```bash
# Check primary site status
ping -c 5 primary-site.company.com

# Check DR site status
ping -c 5 dr-site.company.com

# Verify latest backup
ls -lh /backups/rustydb/ | tail -5
```

**2. Restore to DR Site**:
```bash
# On DR site
sudo systemctl stop rustydb

# Restore latest backup
rusty-db-restore --input /backups/rustydb/latest.backup \
  --data-dir /var/lib/rustydb/instances/default/data \
  --threads 8
```

**3. Apply WAL Archives** (if using PITR):
```bash
# Configure recovery
cat > /var/lib/rustydb/instances/default/data/recovery.conf << EOF
recovery_target = 'immediate'
recovery_target_action = 'promote'
restore_command = 'aws s3 cp s3://rustydb-backup/wal/%f %p'
EOF
```

**4. Start Database**:
```bash
sudo systemctl start rustydb
```

**5. Verify Recovery**:
```bash
# Check database health
rusty-db-cli --command "SELECT * FROM v$database_health;"

# Verify data integrity
rusty-db-cli --command "SELECT COUNT(*) FROM customers;"

# Check latest transaction
rusty-db-cli --command "SELECT MAX(created_at) FROM orders;"
```

**6. Update DNS/Load Balancer**:
```bash
# Update DNS to point to DR site
# This step is application-specific

# Verify connectivity
nslookup database.company.com
```

**7. Monitor Operations**:
```bash
# Watch logs
sudo journalctl -u rustydb -f

# Monitor connections
rusty-db-cli --command "SELECT * FROM v$active_sessions;"
```

### DR Testing

**Quarterly DR Drill Checklist**:

- [ ] Notify stakeholders of DR test
- [ ] Create snapshot of DR database
- [ ] Promote DR to primary (isolated network)
- [ ] Run validation queries
- [ ] Test application connectivity
- [ ] Measure RTO (actual recovery time)
- [ ] Document any issues
- [ ] Revert DR to standby mode
- [ ] Resync DR with production
- [ ] Update DR procedures based on findings

---

## Backup Best Practices

### 1. Regular Testing

- Test restore procedures monthly
- Verify backup integrity weekly
- Full DR drill quarterly

### 2. Offsite Storage

- Keep copies in different geographic locations
- Use cloud storage (S3, Azure Blob, GCS)
- Implement cross-region replication

### 3. Encryption

- Always encrypt backups at rest
- Secure encryption keys (HSM, KMS)
- Regular key rotation

### 4. Monitoring

- Monitor backup completion
- Alert on backup failures
- Track backup size trends
- Monitor backup storage capacity

### 5. Documentation

- Document backup procedures
- Maintain restore runbooks
- Keep contact list updated
- Document all DR tests

### 6. Retention Policy

```
Full backups:     30 days (daily)
Incremental:      7 days
Differential:     7 days
PITR base:        30 days
WAL archives:     Until next full backup
Yearly archival:  7 years (compliance)
```

### 7. Backup Performance

- Use parallel backup threads
- Schedule during low-traffic hours
- Use compression (LZ4 recommended)
- Monitor backup duration trends

### 8. Security

- Restrict backup directory permissions (700)
- Encrypt backup data
- Secure transfer channels (SCP, S3 with TLS)
- Audit backup access

---

**Document Maintained By**: Enterprise Documentation Agent 4
**RustyDB Version**: 0.6.0
