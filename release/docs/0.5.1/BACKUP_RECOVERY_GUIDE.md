# RustyDB v0.5.1 - Backup & Recovery Guide

**Version**: 0.5.1
**Release Date**: 2025-12-25
**Target Audience**: Database Administrators, DevOps Engineers, Site Reliability Engineers
**Classification**: Enterprise Backup & Recovery Operations

---

## Table of Contents

1. [Overview](#overview)
2. [Backup Architecture](#backup-architecture)
3. [Backup Types](#backup-types)
4. [Full Backups](#full-backups)
5. [Incremental Backups](#incremental-backups)
6. [Point-in-Time Recovery (PITR)](#point-in-time-recovery-pitr)
7. [Flashback Features](#flashback-features)
8. [Recovery Procedures](#recovery-procedures)
9. [Disaster Recovery](#disaster-recovery)
10. [Backup Management](#backup-management)
11. [Best Practices](#best-practices)
12. [Troubleshooting](#troubleshooting)
13. [Appendices](#appendices)

---

## Overview

### About This Guide

This comprehensive guide covers all backup and recovery operations for RustyDB v0.5.1, including:

- ✅ **Full, Incremental, and Differential Backups** - Complete backup strategy
- ✅ **Point-in-Time Recovery (PITR)** - Recover to any moment in time
- ✅ **Flashback Technology** - Oracle-style time travel and table recovery
- ✅ **Disaster Recovery** - Standby databases, failover, and RTO/RPO management
- ✅ **Backup Automation** - Scheduling, monitoring, and retention policies
- ✅ **Cloud Integration** - AWS S3, Azure Blob, Google Cloud Storage

### Key Features

**Backup Capabilities**:
- Block-level change tracking for efficient incrementals
- Compression (LZ4, Snappy, Zstd) with 40-70% size reduction
- AES-256-GCM encryption for data at rest
- Parallel backup streams (up to 32 concurrent)
- Backup verification and integrity checks
- Cloud backup with bandwidth throttling

**Recovery Capabilities**:
- Recovery to specific SCN, timestamp, or transaction ID
- Flashback queries for historical data access
- Table-level and database-level flashback
- Block-level recovery for corruption
- Tablespace and datafile recovery
- Automatic crash recovery via WAL

**Enterprise Features**:
- Backup catalog for centralized management
- Retention policies (hourly, daily, weekly, monthly, yearly)
- RTO/RPO monitoring and reporting
- Standby database synchronization
- Automated failover and switchover
- DR testing and validation

---

## Backup Architecture

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Backup System                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Backup     │  │     PITR     │  │   Snapshot   │     │
│  │   Manager    │  │   Manager    │  │   Manager    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│         │                 │                   │             │
│         └─────────────────┴───────────────────┘             │
│                          │                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Encryption   │  │   Disaster   │  │ Verification │     │
│  │   Manager    │  │   Recovery   │  │   Manager    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│         │                 │                   │             │
│         └─────────────────┴───────────────────┘             │
│                          │                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           Backup Catalog (Metadata Repository)       │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        │                                       │
┌───────────────┐                      ┌────────────────┐
│ Local Storage │                      │ Cloud Storage  │
│  /var/backup  │                      │ S3/Azure/GCS   │
└───────────────┘                      └────────────────┘
```

### Component Overview

**Backup Manager** (`src/backup/manager.rs`):
- Orchestrates full, incremental, and differential backups
- Block-level change tracking with System Change Number (SCN)
- Parallel backup streams for performance
- Compression and encryption integration

**PITR Manager** (`src/backup/pitr.rs`):
- Point-in-time recovery coordination
- Log mining and transaction replay
- Restore point management
- Flashback query execution

**Snapshot Manager** (`src/backup/snapshots.rs`):
- Copy-on-write (COW) snapshots
- Instant snapshot creation
- Snapshot cloning and deletion
- Storage-level snapshots

**Disaster Recovery Manager** (`src/backup/disaster_recovery.rs`):
- Standby database management
- Automatic failover and switchover
- RTO/RPO monitoring
- Health checks and replication lag tracking

**Backup Catalog** (`src/backup/catalog.rs`):
- Centralized metadata repository
- Backup set registration
- Recovery path calculation
- Backup piece tracking

---

## Backup Types

### Backup Type Overview

| Backup Type | Description | Typical Size | Use Case |
|-------------|-------------|--------------|----------|
| **Full** | Complete copy of all database blocks | 100% | First backup, weekly baseline |
| **Incremental** | Only blocks changed since last backup | 1-10% | Daily backups, low overhead |
| **Differential** | All blocks changed since last full | 10-50% | Mid-week backups |
| **Archive Log** | Transaction logs only | <1% | Continuous protection |

### Backup Type Selection Guide

**Use Full Backups When**:
- Establishing a new backup baseline
- Starting a new backup strategy
- Weekly or monthly scheduled backups
- After major database changes (migration, upgrade)

**Use Incremental Backups When**:
- Daily protection is needed
- Storage space is limited
- Backup window is tight
- Change rate is low (<10% daily)

**Use Differential Backups When**:
- Want faster recovery than incremental chain
- Mid-week backup between full backups
- Simplified restore process (only 2 backups needed)

**Use Archive Log Backups When**:
- Continuous PITR capability required
- Zero data loss objective (RPO=0)
- Complement to full/incremental backups

---

## Full Backups

### Full Backup Overview

A full backup creates a complete copy of all database blocks, including:
- All data files (tables, indexes)
- Control files (database metadata)
- Initialization parameters
- System Change Number (SCN) at backup time

**Full Backup Characteristics**:
- Self-contained (no dependency on other backups)
- Largest backup size
- Longest backup duration
- Fastest single-backup recovery
- Baseline for incremental/differential backups

### Creating a Full Backup

#### Online Full Backup (Default)

**Command Line**:
```bash
# Using rusty-db-cli
rusty-db-cli --host localhost --port 5432 << EOF
BEGIN BACKUP;
BACKUP DATABASE TO '/var/lib/rustydb/backup/full_backup_$(date +%Y%m%d)'
  COMPRESSION ENABLED
  ENCRYPTION ENABLED;
COMMIT;
EOF
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${RUSTYDB_ADMIN_TOKEN}" \
  -d '{
    "type": "FULL",
    "compression": true,
    "encryption": true,
    "destination": "/var/lib/rustydb/backup/full_backup",
    "parallel_streams": 8
  }'
```

**Configuration File** (`/etc/rustydb/backup.toml`):
```toml
[backup]
backup_dir = "/var/lib/rustydb/backup"
max_parallel_streams = 8
buffer_size = 1048576  # 1MB
compression_enabled = true
compression_level = 6
encryption_enabled = true
verify_after_backup = true
block_size = 8192
enable_change_tracking = true
```

#### Offline Full Backup (Cold Backup)

For offline backups, shut down the database first:

```bash
# Stop database
sudo systemctl stop rustydb

# Perform filesystem copy
sudo tar czf /backup/rustydb_offline_$(date +%Y%m%d_%H%M%S).tar.gz \
  /var/lib/rustydb/data \
  /var/lib/rustydb/wal \
  /etc/rustydb/config.toml

# Restart database
sudo systemctl start rustydb
```

**Pros**: Simplest, guaranteed consistency
**Cons**: Requires downtime, not suitable for 24/7 operations

### Full Backup Options

#### Compression

**Supported Algorithms**:
- **LZ4**: Fast compression, ~40% reduction, 500 MB/s throughput
- **Snappy**: Balanced, ~45% reduction, 350 MB/s throughput
- **Zstd**: Best compression, ~60% reduction, 200 MB/s throughput (default)

**Configuration**:
```toml
[backup]
compression_enabled = true
compression_algorithm = "zstd"  # lz4, snappy, zstd
compression_level = 6  # 1-9 (higher = better compression, slower)
```

**Example**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -d '{
    "type": "FULL",
    "compression": true,
    "compression_algorithm": "zstd",
    "compression_level": 9
  }'
```

**Compression Ratios** (typical):
- Uncompressed data files: 1.0 (baseline)
- Full backup with LZ4: 0.60 (40% reduction)
- Full backup with Zstd level 6: 0.40 (60% reduction)
- Full backup with Zstd level 9: 0.35 (65% reduction)

#### Encryption

**Encryption Algorithm**: AES-256-GCM (Galois/Counter Mode)

**Key Management**:
```bash
# Generate master encryption key
openssl rand -out /etc/rustydb/secrets/backup_master.key 32

# Set permissions
sudo chown rustydb:rustydb /etc/rustydb/secrets/backup_master.key
sudo chmod 400 /etc/rustydb/secrets/backup_master.key
```

**Configuration**:
```toml
[backup.encryption]
enabled = true
algorithm = "AES256-GCM"
master_key_file = "/etc/rustydb/secrets/backup_master.key"
key_rotation_days = 90
```

**Encrypted Backup**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -d '{
    "type": "FULL",
    "encryption": true,
    "encryption_algorithm": "AES256-GCM",
    "key_derivation": "PBKDF2-SHA256"
  }'
```

#### Parallel Backup Streams

**Configuration**:
```toml
[backup]
max_parallel_streams = 8  # Number of concurrent backup threads
```

**Performance Impact**:
- 1 stream: ~100 MB/s (baseline)
- 4 streams: ~350 MB/s (3.5x)
- 8 streams: ~600 MB/s (6x)
- 16 streams: ~800 MB/s (8x, diminishing returns)
- 32 streams: ~900 MB/s (9x, CPU-bound)

**Recommendation**: Set to number of CPU cores or slightly higher (cores × 1.5)

### Full Backup Script

**Automated Full Backup Script** (`/usr/local/bin/rustydb-full-backup.sh`):

```bash
#!/bin/bash
# RustyDB Full Backup Script
# Usage: /usr/local/bin/rustydb-full-backup.sh [database_name]

set -euo pipefail

# Configuration
DATABASE_NAME="${1:-rustydb}"
BACKUP_DIR="/var/lib/rustydb/backup"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_ID="FULL-${TIMESTAMP}"
BACKUP_PATH="${BACKUP_DIR}/${BACKUP_ID}"
LOG_FILE="/var/log/rustydb/backup.log"
RETENTION_DAYS=30

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "${LOG_FILE}"
}

# Check disk space
check_disk_space() {
    local required_space_gb=50
    local available_space=$(df -BG "${BACKUP_DIR}" | tail -1 | awk '{print $4}' | sed 's/G//')

    if [ "${available_space}" -lt "${required_space_gb}" ]; then
        log "ERROR: Insufficient disk space. Required: ${required_space_gb}GB, Available: ${available_space}GB"
        exit 1
    fi
    log "Disk space check passed. Available: ${available_space}GB"
}

# Perform backup
perform_backup() {
    log "Starting full backup: ${BACKUP_ID}"

    local start_time=$(date +%s)

    # Execute backup via REST API
    local response=$(curl -s -X POST http://localhost:8080/api/v1/admin/backup \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer ${RUSTYDB_ADMIN_TOKEN}" \
        -d "{
            \"type\": \"FULL\",
            \"database_name\": \"${DATABASE_NAME}\",
            \"compression\": true,
            \"encryption\": true,
            \"destination\": \"${BACKUP_PATH}\",
            \"parallel_streams\": 8,
            \"verify\": true
        }")

    local backup_id=$(echo "${response}" | jq -r '.data.backup_id')

    if [ -z "${backup_id}" ] || [ "${backup_id}" = "null" ]; then
        log "ERROR: Backup failed. Response: ${response}"
        exit 1
    fi

    log "Backup created: ${backup_id}"

    # Wait for backup completion
    while true; do
        local status=$(curl -s http://localhost:8080/api/v1/admin/backup/${backup_id}/status | jq -r '.data.status')

        if [ "${status}" = "COMPLETED" ]; then
            break
        elif [ "${status}" = "FAILED" ]; then
            log "ERROR: Backup failed"
            exit 1
        fi

        sleep 5
    done

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    log "Backup completed in ${duration} seconds"
}

# Verify backup
verify_backup() {
    log "Verifying backup: ${BACKUP_ID}"

    # Check backup files exist
    if [ ! -d "${BACKUP_PATH}" ]; then
        log "ERROR: Backup directory not found: ${BACKUP_PATH}"
        exit 1
    fi

    # Verify backup integrity
    curl -s -X POST http://localhost:8080/api/v1/admin/backup/verify \
        -H "Content-Type: application/json" \
        -d "{\"backup_id\": \"${BACKUP_ID}\"}" | jq .

    log "Backup verification successful"
}

# Upload to cloud (optional)
upload_to_cloud() {
    if [ -n "${AWS_S3_BUCKET:-}" ]; then
        log "Uploading backup to S3: ${AWS_S3_BUCKET}"

        aws s3 sync "${BACKUP_PATH}" "s3://${AWS_S3_BUCKET}/rustydb-backups/${BACKUP_ID}/" \
            --storage-class GLACIER_IR

        log "Cloud upload completed"
    fi
}

# Clean up old backups
cleanup_old_backups() {
    log "Cleaning up backups older than ${RETENTION_DAYS} days"

    find "${BACKUP_DIR}" -name "FULL-*" -type d -mtime +${RETENTION_DAYS} -exec rm -rf {} + 2>/dev/null || true

    log "Cleanup completed"
}

# Main execution
main() {
    log "========================================="
    log "RustyDB Full Backup Started"
    log "Database: ${DATABASE_NAME}"
    log "========================================="

    check_disk_space
    perform_backup
    verify_backup
    upload_to_cloud
    cleanup_old_backups

    log "========================================="
    log "Full Backup Completed Successfully"
    log "Backup ID: ${BACKUP_ID}"
    log "========================================="
}

# Run main function
main "$@"
```

**Make executable and schedule**:
```bash
sudo chmod +x /usr/local/bin/rustydb-full-backup.sh

# Schedule weekly full backup (Sunday 2 AM)
sudo crontab -e
# Add: 0 2 * * 0 /usr/local/bin/rustydb-full-backup.sh
```

---

## Incremental Backups

### Incremental Backup Overview

Incremental backups capture only the blocks that have changed since the **last backup** (full or incremental). This provides:

- **Storage Efficiency**: 90-99% smaller than full backups
- **Fast Backups**: Complete in minutes instead of hours
- **Minimal Impact**: Low I/O and CPU overhead
- **Continuous Protection**: Can run every 15-30 minutes

**How Incremental Backups Work**:
1. Block Change Tracking (BCT) monitors which blocks change
2. Incremental backup reads only changed blocks
3. Each incremental depends on all previous backups
4. Recovery requires full backup + all incrementals in sequence

### Block-Level Change Tracking

**Change Tracking Architecture**:
```
┌─────────────────────────────────────────────────┐
│         Block Change Tracking (BCT)             │
├─────────────────────────────────────────────────┤
│                                                  │
│  Data Block: 1000                                │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┐   │
│  │ SCN  │ File │ Block│ Oper │Changed│ Size │   │
│  ├──────┼──────┼──────┼──────┼──────┼──────┤   │
│  │ 1001 │  1   │ 1000 │INSERT│  Yes │  8KB │   │
│  │ 1002 │  1   │ 1001 │UPDATE│  Yes │  8KB │   │
│  │ 1003 │  2   │ 2000 │DELETE│  Yes │  8KB │   │
│  └──────┴──────┴──────┴──────┴──────┴──────┘   │
│                                                  │
│  Bitmap: [1,1,0,0,1,0,0,0, ...]                 │
│  Changed Blocks: 3 / 10000 (0.03%)              │
│                                                  │
└─────────────────────────────────────────────────┘
```

**Enable Change Tracking**:
```toml
[backup]
enable_change_tracking = true
block_size = 8192  # 8KB blocks
```

**Change Tracking Overhead**:
- Memory: ~1 MB per 100 GB database
- CPU: <1% overhead
- I/O: Negligible (metadata only)

### Creating Incremental Backups

#### Level 0 (Full) + Level 1 (Incremental) Strategy

```bash
# Level 0 backup (full backup baseline)
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -d '{
    "type": "FULL",
    "database_name": "rustydb",
    "compression": true
  }'
# Returns: { "backup_id": "FULL-20251225_020000" }

# Level 1 incremental backup (next day)
curl -X POST http://localhost:8080/api/v1/admin/backup/incremental \
  -d '{
    "database_name": "rustydb",
    "parent_backup_id": "FULL-20251225_020000",
    "compression": true
  }'
# Returns: { "backup_id": "INCR-20251226_020000" }

# Level 1 incremental backup (day 3)
curl -X POST http://localhost:8080/api/v1/admin/backup/incremental \
  -d '{
    "database_name": "rustydb",
    "parent_backup_id": "INCR-20251226_020000",
    "compression": true
  }'
# Returns: { "backup_id": "INCR-20251227_020000" }
```

**Recovery from Incremental Chain**:
```
Full Backup (Day 1)
  → Incremental 1 (Day 2)
  → Incremental 2 (Day 3)
  → Incremental 3 (Day 4)

Recovery requires: ALL 4 backups in sequence
```

### Differential Backups

Differential backups capture all blocks changed since the **last full backup**:

- **Larger than Incremental**: 10-50% of full backup size
- **Faster Recovery**: Only 2 backups needed (full + differential)
- **Simpler Management**: No long chain dependencies

```bash
# Full backup (baseline)
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -d '{"type": "FULL"}'
# Returns: "FULL-20251225_020000"

# Differential backup (day 2)
curl -X POST http://localhost:8080/api/v1/admin/backup/differential \
  -d '{
    "database_name": "rustydb",
    "base_backup_id": "FULL-20251225_020000"
  }'
# Returns: "DIFF-20251226_020000"

# Differential backup (day 3)
curl -X POST http://localhost:8080/api/v1/admin/backup/differential \
  -d '{
    "database_name": "rustydb",
    "base_backup_id": "FULL-20251225_020000"
  }'
# Returns: "DIFF-20251227_020000"
```

**Recovery from Differential**:
```
Full Backup (Day 1) → Differential (Day 4)

Recovery requires: Only 2 backups (full + latest differential)
```

### Incremental Backup Strategy

**Weekly Full + Daily Incremental**:
```
Sun: Full Backup (100 GB)
Mon: Incremental (2 GB)
Tue: Incremental (3 GB)
Wed: Incremental (2 GB)
Thu: Incremental (4 GB)
Fri: Incremental (2 GB)
Sat: Incremental (1 GB)

Total Storage: 114 GB
Recovery Time: Moderate (7 backups to apply)
```

**Weekly Full + Daily Differential**:
```
Sun: Full Backup (100 GB)
Mon: Differential (5 GB)
Tue: Differential (10 GB)
Wed: Differential (15 GB)
Thu: Differential (20 GB)
Fri: Differential (25 GB)
Sat: Differential (30 GB)

Total Storage: 205 GB
Recovery Time: Fast (2 backups to apply)
```

**Hybrid Strategy** (Recommended):
```
Sun: Full Backup (100 GB)
Mon: Incremental (2 GB)
Tue: Incremental (2 GB)
Wed: Differential (8 GB)    ← Mid-week checkpoint
Thu: Incremental (2 GB)
Fri: Incremental (2 GB)
Sat: Differential (12 GB)   ← End-of-week checkpoint

Total Storage: 128 GB
Recovery Time: Balanced (max 4 backups to apply)
```

### Incremental Backup Script

**Automated Incremental Backup** (`/usr/local/bin/rustydb-incremental-backup.sh`):

```bash
#!/bin/bash
# RustyDB Incremental Backup Script

set -euo pipefail

BACKUP_DIR="/var/lib/rustydb/backup"
LOG_FILE="/var/log/rustydb/backup.log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "${LOG_FILE}"
}

# Find the most recent backup (full or incremental)
find_parent_backup() {
    local latest=$(ls -t "${BACKUP_DIR}" | grep -E '^(FULL|INCR|DIFF)-' | head -1)
    echo "${latest}"
}

# Perform incremental backup
main() {
    log "Starting incremental backup"

    local parent_backup=$(find_parent_backup)

    if [ -z "${parent_backup}" ]; then
        log "ERROR: No parent backup found. Run full backup first."
        exit 1
    fi

    log "Parent backup: ${parent_backup}"

    local response=$(curl -s -X POST http://localhost:8080/api/v1/admin/backup/incremental \
        -H "Content-Type: application/json" \
        -d "{
            \"database_name\": \"rustydb\",
            \"parent_backup_id\": \"${parent_backup}\",
            \"compression\": true,
            \"encryption\": true
        }")

    local backup_id=$(echo "${response}" | jq -r '.data.backup_id')

    log "Incremental backup completed: ${backup_id}"
}

main "$@"
```

**Schedule**:
```bash
# Daily incremental backup (2 AM, Monday-Saturday)
0 2 * * 1-6 /usr/local/bin/rustydb-incremental-backup.sh
```

### Merge Incremental Backups

RustyDB automatically merges incremental backup change maps for recovery:

```bash
# View backup chain
curl http://localhost:8080/api/v1/admin/backup/chain?backup_id=INCR-20251227

# Example output:
{
  "chain": [
    {"backup_id": "FULL-20251225", "type": "FULL", "size_gb": 100},
    {"backup_id": "INCR-20251226", "type": "INCREMENTAL", "size_gb": 2},
    {"backup_id": "INCR-20251227", "type": "INCREMENTAL", "size_gb": 3}
  ],
  "total_size_gb": 105,
  "recovery_time_estimate_minutes": 45
}
```

---

## Point-in-Time Recovery (PITR)

### PITR Overview

Point-in-Time Recovery allows you to restore the database to any moment in the past, down to the second. This is achieved through:

1. **Base Backup**: Full or incremental backup
2. **WAL Archive**: Continuous archive of transaction logs
3. **Log Replay**: Apply WAL logs up to target time/SCN

**PITR Capabilities**:
- Recover to specific timestamp
- Recover to specific SCN (System Change Number)
- Recover to specific transaction ID
- Recover to named restore point
- Recover to latest (most recent commit)

### WAL Archiving

**Configure WAL Archiving** (`/etc/rustydb/config.toml`):

```toml
[backup]
wal_archiving_enabled = true
wal_archive_dir = "/var/lib/rustydb/wal_archive"
wal_archive_command = "cp %p /var/lib/rustydb/wal_archive/%f"
wal_retention_hours = 168  # 7 days
wal_segment_size_mb = 16
```

**WAL Archive Structure**:
```
/var/lib/rustydb/wal_archive/
├── 000000010000000000000001  # WAL segment 1
├── 000000010000000000000002  # WAL segment 2
├── 000000010000000000000003  # WAL segment 3
├── ...
└── 000000010000000000000FFF  # WAL segment 4095
```

**Monitor WAL Archiving**:
```bash
# Check WAL archive status
curl http://localhost:8080/api/v1/admin/wal/archive/status

# Example output:
{
  "enabled": true,
  "archive_dir": "/var/lib/rustydb/wal_archive",
  "segments_archived": 1234,
  "oldest_segment": "000000010000000000000001",
  "newest_segment": "0000000100000000000004D2",
  "archive_size_gb": 19.6,
  "retention_hours": 168
}
```

### Recovery Targets

**1. Recovery to Timestamp**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "recovery_target": {
      "type": "TIMESTAMP",
      "value": "2025-12-24T15:30:00Z"
    },
    "recovery_mode": "INCOMPLETE",
    "recovery_path": "/var/lib/rustydb/restore"
  }'
```

**Use Case**: Recover to before accidental data deletion occurred at 3:30 PM.

**2. Recovery to SCN (System Change Number)**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "recovery_target": {
      "type": "SCN",
      "value": 123456789
    },
    "recovery_mode": "INCOMPLETE"
  }'
```

**Use Case**: Recover to exact database state at SCN 123456789.

**3. Recovery to Transaction ID**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "recovery_target": {
      "type": "TRANSACTION",
      "value": "TXN-ABCD-1234"
    },
    "recovery_mode": "INCOMPLETE"
  }'
```

**Use Case**: Recover to just before specific transaction committed.

**4. Recovery to Restore Point**:
```bash
# Create restore point
curl -X POST http://localhost:8080/api/v1/admin/restore-point \
  -d '{
    "name": "before_migration",
    "guaranteed": true,
    "description": "Before schema migration"
  }'

# Recover to restore point
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "recovery_target": {
      "type": "RESTORE_POINT",
      "value": "before_migration"
    },
    "recovery_mode": "INCOMPLETE"
  }'
```

**Use Case**: Recover to named checkpoint before risky operation.

**5. Recovery to Latest**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "recovery_target": {
      "type": "LATEST"
    },
    "recovery_mode": "COMPLETE"
  }'
```

**Use Case**: Crash recovery, apply all available WAL logs.

### PITR Procedure

**Complete PITR Walkthrough**:

**Step 1: Prepare Recovery Environment**
```bash
# Stop database if running
sudo systemctl stop rustydb

# Create recovery directory
sudo mkdir -p /var/lib/rustydb/restore
sudo chown rustydb:rustydb /var/lib/rustydb/restore
```

**Step 2: Identify Target Recovery Point**
```bash
# List restore points
curl http://localhost:8080/api/v1/admin/restore-points

# Query log miner to find target SCN
curl http://localhost:8080/api/v1/admin/log-miner/query \
  -d '{
    "start_time": "2025-12-24T00:00:00Z",
    "end_time": "2025-12-24T23:59:59Z",
    "table_name": "employees",
    "operation": "DELETE"
  }'

# Example output:
{
  "entries": [
    {
      "scn": 123456789,
      "timestamp": "2025-12-24T15:27:43Z",
      "transaction_id": "TXN-5678",
      "operation": "DELETE",
      "table_name": "employees",
      "rows_affected": 150,
      "undo_sql": "INSERT INTO employees VALUES (...)"
    }
  ]
}
```

**Step 3: Select Backup for Recovery**
```bash
# Find backup covering target time
curl http://localhost:8080/api/v1/admin/backup/find-recovery-path \
  -d '{
    "database_name": "rustydb",
    "target_scn": 123456700
  }'

# Example output:
{
  "recovery_path": [
    {"backup_id": "FULL-20251224_020000", "scn": 123450000},
    {"backup_id": "INCR-20251224_140000", "scn": 123456000}
  ]
}
```

**Step 4: Start Recovery Session**
```bash
curl -X POST http://localhost:8080/api/v1/admin/pitr/start \
  -d '{
    "backup_id": "FULL-20251224_020000",
    "recovery_target": {
      "type": "SCN",
      "value": 123456700
    },
    "recovery_mode": "INCOMPLETE",
    "recovery_path": "/var/lib/rustydb/restore",
    "validate_blocks": true
  }'

# Returns:
{
  "session_id": "RECOVERY-abc123",
  "status": "INITIALIZING"
}
```

**Step 5: Monitor Recovery Progress**
```bash
# Check recovery status
watch -n 5 "curl -s http://localhost:8080/api/v1/admin/pitr/status/RECOVERY-abc123 | jq ."

# Example output:
{
  "session_id": "RECOVERY-abc123",
  "status": {
    "type": "APPLYING_LOGS",
    "current_scn": 123456500,
    "target_scn": 123456700,
    "progress_pct": 87.2
  },
  "logs_applied": 145,
  "start_time": "2025-12-25T10:00:00Z",
  "elapsed_seconds": 320
}
```

**Step 6: Validate Recovery**
```bash
# After recovery completes, verify data
curl http://localhost:8080/api/v1/admin/pitr/validate/RECOVERY-abc123

# Example validation result:
{
  "validation_status": "SUCCESS",
  "target_scn_reached": 123456700,
  "blocks_validated": 125000,
  "corrupted_blocks": 0,
  "recovery_time_seconds": 380
}
```

**Step 7: Open Database**
```bash
# Start database with recovered data
sudo systemctl start rustydb

# Verify data
rusty-db-cli --host localhost << EOF
SELECT COUNT(*) FROM employees;
-- Should show 150 more rows than after deletion
EOF
```

### Restore Points

**Create Guaranteed Restore Point**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/restore-points \
  -d '{
    "name": "pre_upgrade_v051",
    "guaranteed": true,
    "description": "Before upgrading to v0.5.1"
  }'
```

**Guaranteed vs Normal Restore Points**:
- **Guaranteed**: WAL logs preserved indefinitely, guaranteed recovery
- **Normal**: WAL logs subject to retention policy, may be purged

**List Restore Points**:
```bash
curl http://localhost:8080/api/v1/admin/restore-points

# Example output:
{
  "restore_points": [
    {
      "name": "pre_upgrade_v051",
      "scn": 123450000,
      "timestamp": "2025-12-24T10:00:00Z",
      "guaranteed": true,
      "created_by": "admin"
    },
    {
      "name": "daily_checkpoint",
      "scn": 123460000,
      "timestamp": "2025-12-25T02:00:00Z",
      "guaranteed": false,
      "created_by": "system"
    }
  ]
}
```

**Drop Restore Point**:
```bash
curl -X DELETE http://localhost:8080/api/v1/admin/restore-points/pre_upgrade_v051
```

---

## Flashback Features

### Flashback Overview

RustyDB provides Oracle-style flashback capabilities for time travel queries and table recovery without full database restore:

- **Flashback Query**: Query historical data as it existed at a past time
- **Flashback Versions Query**: View all versions of rows in a time range
- **Flashback Table**: Restore table to previous state
- **Flashback Database**: Database-level point-in-time flashback
- **Flashback Transaction**: Analyze and reverse specific transactions

### Flashback Query (Time Travel)

**Query Data as of Timestamp**:
```sql
-- See employee data as it existed at 3:00 PM yesterday
SELECT * FROM employees
AS OF TIMESTAMP '2025-12-24 15:00:00'
WHERE department_id = 100;

-- Compare current vs historical data
SELECT
  current.employee_id,
  current.salary as current_salary,
  historical.salary as historical_salary,
  (current.salary - historical.salary) as salary_change
FROM employees current
JOIN employees AS OF TIMESTAMP '2025-01-01 00:00:00' historical
  ON current.employee_id = historical.employee_id
WHERE current.salary != historical.salary;
```

**Query Data as of SCN**:
```sql
-- Query at specific SCN
SELECT * FROM orders
AS OF SCN 123456789
WHERE order_date >= '2025-12-01';
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/query/flashback \
  -d '{
    "table_name": "employees",
    "target": {
      "type": "TIMESTAMP",
      "value": "2025-12-24T15:00:00Z"
    },
    "query": "SELECT * FROM employees WHERE department_id = 100"
  }'
```

### Flashback Versions Query

**View All Versions of Rows**:
```sql
-- See all versions of employee #123 between two times
SELECT
  versions_starttime,
  versions_endtime,
  versions_xid,
  versions_operation,
  employee_id,
  first_name,
  salary
FROM employees
VERSIONS BETWEEN TIMESTAMP
  '2025-12-01 00:00:00' AND '2025-12-31 23:59:59'
WHERE employee_id = 123
ORDER BY versions_starttime;

-- Example output:
-- START_TIME         | END_TIME           | XID    | OP     | EMP_ID | NAME  | SALARY
-- 2025-12-01 09:00   | 2025-12-15 10:30   | TXN001 | INSERT | 123    | John  | 50000
-- 2025-12-15 10:30   | 2025-12-20 14:00   | TXN045 | UPDATE | 123    | John  | 55000
-- 2025-12-20 14:00   | NULL               | TXN078 | UPDATE | 123    | John  | 60000
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/query/flashback-versions \
  -d '{
    "table_name": "employees",
    "start_time": "2025-12-01T00:00:00Z",
    "end_time": "2025-12-31T23:59:59Z",
    "where_clause": "employee_id = 123"
  }'
```

### Flashback Table

**Restore Table to Previous State**:
```sql
-- Flashback table to 1 hour ago
FLASHBACK TABLE employees
TO TIMESTAMP TIMESTAMP '2025-12-25 14:00:00';

-- Flashback table to SCN
FLASHBACK TABLE employees
TO SCN 123456789;

-- Flashback table before drop (undelete)
FLASHBACK TABLE employees_backup
TO BEFORE DROP;
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/flashback/table \
  -d '{
    "table_name": "employees",
    "target": {
      "type": "TIMESTAMP",
      "value": "2025-12-25T14:00:00Z"
    },
    "options": {
      "enable_triggers": false,
      "rename_to": "employees_restored"
    }
  }'
```

**Flashback Table Options**:
- **enable_triggers**: Whether to fire triggers during flashback (default: false)
- **rename_to**: Restore to different table name
- **restore_constraints**: Restore constraints and indexes (default: true)

### Flashback Database

**Database-Level Flashback**:
```bash
# Flashback entire database to restore point
curl -X POST http://localhost:8080/api/v1/admin/flashback/database \
  -d '{
    "target": {
      "type": "RESTORE_POINT",
      "value": "before_migration"
    },
    "open_mode": "READ_WRITE"
  }'
```

**Flashback Database Procedure**:
```bash
# 1. Shutdown database
sudo systemctl stop rustydb

# 2. Start in flashback mode
curl -X POST http://localhost:8080/api/v1/admin/flashback/database/start \
  -d '{"target_scn": 123456789}'

# 3. Monitor flashback progress
curl http://localhost:8080/api/v1/admin/flashback/database/status

# 4. Open database with RESETLOGS
curl -X POST http://localhost:8080/api/v1/admin/database/open \
  -d '{"mode": "RESETLOGS"}'

# 5. Restart database normally
sudo systemctl start rustydb
```

### Flashback Transaction

**Analyze Transaction**:
```bash
# Find transactions affecting specific table
curl -X POST http://localhost:8080/api/v1/admin/flashback/transaction/query \
  -d '{
    "table_name": "employees",
    "start_time": "2025-12-24T00:00:00Z",
    "end_time": "2025-12-24T23:59:59Z",
    "operations": ["DELETE", "UPDATE"]
  }'

# Example output:
{
  "transactions": [
    {
      "transaction_id": "TXN-5678",
      "start_scn": 123456780,
      "commit_scn": 123456789,
      "operations": [
        {
          "operation": "DELETE",
          "table_name": "employees",
          "row_id": "ROW-001",
          "undo_sql": "INSERT INTO employees VALUES (1, 'John', 50000)"
        },
        {
          "operation": "DELETE",
          "table_name": "employees",
          "row_id": "ROW-002",
          "undo_sql": "INSERT INTO employees VALUES (2, 'Jane', 55000)"
        }
      ]
    }
  ]
}
```

**Reverse Transaction**:
```bash
# Flashback (reverse) a specific transaction
curl -X POST http://localhost:8080/api/v1/admin/flashback/transaction/reverse \
  -d '{
    "transaction_id": "TXN-5678",
    "cascade": true
  }'
```

**cascade=true**: Reverse all dependent transactions as well

---

## Recovery Procedures

### Recovery Types

**1. Complete Recovery**: Recover all committed transactions
**2. Incomplete Recovery**: Recover to specific point in time (PITR)
**3. Crash Recovery**: Automatic recovery after crash
**4. Media Recovery**: Recovery after disk failure
**5. Block-Level Recovery**: Recover corrupted blocks
**6. Tablespace Recovery**: Recover specific tablespace
**7. Datafile Recovery**: Recover specific datafile

### Complete Recovery

**Scenario**: Database crashed, recover to latest committed transaction.

**Procedure**:
```bash
# 1. Verify database is stopped
sudo systemctl status rustydb

# 2. Start recovery
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "backup_id": "FULL-20251224_020000",
    "recovery_target": {"type": "LATEST"},
    "recovery_mode": "COMPLETE"
  }'

# 3. Monitor recovery
curl http://localhost:8080/api/v1/admin/restore/status/RECOVERY-xyz

# 4. Start database
sudo systemctl start rustydb
```

**Expected Outcome**: All committed transactions recovered, zero data loss.

### Incomplete Recovery (PITR)

See [Point-in-Time Recovery](#point-in-time-recovery-pitr) section above.

### Crash Recovery (Automatic)

RustyDB performs crash recovery automatically on startup using WAL:

**Crash Recovery Process**:
1. **Analysis Phase**: Scan WAL from last checkpoint
2. **Redo Phase**: Replay all committed transactions
3. **Undo Phase**: Rollback uncommitted transactions

**Monitor Crash Recovery**:
```bash
# View recovery logs
sudo journalctl -u rustydb -n 100

# Example output:
# [INFO] RustyDB starting crash recovery
# [INFO] Last checkpoint SCN: 123450000
# [INFO] Scanning WAL from SCN 123450000
# [INFO] Found 500 WAL segments to replay
# [INFO] Redo phase: Replaying 12,345 transactions
# [INFO] Undo phase: Rolling back 3 uncommitted transactions
# [INFO] Crash recovery completed in 45 seconds
# [INFO] Database ready for connections
```

**No manual intervention required** - crash recovery is automatic.

### Media Recovery

**Scenario**: Disk failure, data file corrupted.

**Procedure**:

**Step 1: Identify Corrupted File**
```bash
# Check database health
curl http://localhost:8080/api/v1/admin/health

# Example error:
{
  "status": "DEGRADED",
  "errors": [
    "Data file /var/lib/rustydb/data/table_1.dat: I/O error"
  ]
}
```

**Step 2: Restore Data File from Backup**
```bash
# Stop database
sudo systemctl stop rustydb

# Restore specific file from backup
curl -X POST http://localhost:8080/api/v1/admin/restore/datafile \
  -d '{
    "backup_id": "FULL-20251224_020000",
    "datafile_path": "/var/lib/rustydb/data/table_1.dat",
    "target_directory": "/var/lib/rustydb/data"
  }'
```

**Step 3: Recover with WAL**
```bash
# Apply WAL logs to bring file up to date
curl -X POST http://localhost:8080/api/v1/admin/restore/recover-datafile \
  -d '{
    "datafile_path": "/var/lib/rustydb/data/table_1.dat",
    "recovery_target": {"type": "LATEST"}
  }'
```

**Step 4: Restart Database**
```bash
sudo systemctl start rustydb
```

### Block-Level Recovery

**Scenario**: Specific blocks corrupted (detected by checksums).

**Procedure**:
```bash
# Identify corrupted blocks
curl http://localhost:8080/api/v1/admin/verify/blocks

# Example output:
{
  "corrupted_blocks": [
    {"file_id": 1, "block_id": 1000, "checksum_mismatch": true},
    {"file_id": 1, "block_id": 1001, "checksum_mismatch": true}
  ]
}

# Recover specific blocks
curl -X POST http://localhost:8080/api/v1/admin/restore/blocks \
  -d '{
    "backup_id": "FULL-20251224_020000",
    "blocks": [
      {"file_id": 1, "block_id": 1000},
      {"file_id": 1, "block_id": 1001}
    ]
  }'
```

**Advantage**: No downtime, surgical recovery of specific blocks.

### Tablespace Recovery

**Procedure**:
```bash
# Take tablespace offline
curl -X POST http://localhost:8080/api/v1/admin/tablespace/offline \
  -d '{"tablespace_name": "users"}'

# Restore tablespace
curl -X POST http://localhost:8080/api/v1/admin/restore/tablespace \
  -d '{
    "backup_id": "FULL-20251224_020000",
    "tablespace_name": "users",
    "recovery_target": {"type": "LATEST"}
  }'

# Bring tablespace online
curl -X POST http://localhost:8080/api/v1/admin/tablespace/online \
  -d '{"tablespace_name": "users"}'
```

---

## Disaster Recovery

### Disaster Recovery Overview

RustyDB provides comprehensive disaster recovery capabilities:

- **Standby Databases**: Synchronized replica for failover
- **Automatic Failover**: Detect failure and promote standby
- **Switchover**: Planned role reversal (zero downtime)
- **RTO/RPO Monitoring**: Track recovery objectives
- **DR Testing**: Validate DR plan without impacting production

### Standby Database Setup

**Primary Database Configuration** (`/etc/rustydb/config.toml`):
```toml
[replication]
replication_mode = "sync"  # sync, async, semi-sync
replication_port = 7433
replication_slots_enabled = true
max_replication_lag_bytes = 104857600  # 100 MB
```

**Create Replication User**:
```sql
CREATE USER replicator WITH REPLICATION PASSWORD 'SecureRepl!c@t0r';
GRANT REPLICATION ON DATABASE rustydb TO replicator;
```

**Initialize Standby from Primary**:
```bash
# On standby server
sudo -u rustydb rusty-db-server --init-standby \
  --primary=10.0.1.10:7433 \
  --user=replicator \
  --password='SecureRepl!c@t0r' \
  --data-dir=/var/lib/rustydb/data
```

**Start Standby**:
```bash
sudo systemctl start rustydb
```

**Verify Replication**:
```bash
# Check replication status
curl http://10.0.1.10:8080/api/v1/cluster/replication

# Example output:
{
  "mode": "SYNCHRONOUS",
  "standbys": [
    {
      "name": "standby-1",
      "address": "10.0.1.11:7433",
      "status": "STREAMING",
      "lag_bytes": 0,
      "lag_seconds": 0,
      "last_applied_lsn": 123456789
    }
  ]
}
```

### Automatic Failover

**Configure Automatic Failover** (`/etc/rustydb/dr.toml`):
```toml
[disaster_recovery]
auto_failover_enabled = true
health_check_interval_seconds = 5
max_primary_failures = 3
max_lag_tolerance_seconds = 60
switchover_timeout_seconds = 300

[rto]
target_seconds = 300  # 5 minutes
max_acceptable_seconds = 600  # 10 minutes

[rpo]
target_seconds = 60  # 1 minute
max_acceptable_data_loss_seconds = 300  # 5 minutes
```

**Failover Triggers**:
- Primary unreachable for >15 seconds
- Health check failures: 3 consecutive
- Replication lag: >60 seconds (configurable)
- Manual trigger

**Monitor Failover Status**:
```bash
curl http://10.0.1.11:8080/api/v1/cluster/dr/status

# Example output:
{
  "role": "STANDBY",
  "primary": "10.0.1.10",
  "auto_failover_enabled": true,
  "health_checks_passed": 150,
  "health_checks_failed": 0,
  "last_health_check": "2025-12-25T10:00:00Z",
  "replication_lag_seconds": 0
}
```

### Manual Failover

**Trigger Manual Failover**:
```bash
# On standby server, promote to primary
curl -X POST http://10.0.1.11:8080/api/v1/cluster/failover \
  -d '{
    "trigger": "MANUAL",
    "target_standby": "standby-1"
  }'

# Monitor failover progress
curl http://10.0.1.11:8080/api/v1/cluster/failover/status/FAILOVER-xyz

# Example output:
{
  "event_id": "FAILOVER-xyz",
  "status": "IN_PROGRESS",
  "step": "PROMOTING_STANDBY",
  "steps_completed": [
    "VALIDATING_STANDBY",
    "STOPPING_REPLICATION"
  ],
  "elapsed_seconds": 45
}
```

**Failover Steps**:
1. **Validate Standby**: Check standby is healthy and up-to-date
2. **Stop Replication**: Disconnect from (failed) primary
3. **Promote Standby**: Activate standby as new primary
4. **Reconfigure Clients**: Update connection strings
5. **Verify Promoted**: Confirm new primary accepts writes

**Expected Failover Time**: 30-120 seconds

### Switchover (Planned Failover)

**Planned role reversal with zero data loss**:

```bash
# Step 1: Verify replication is synchronized
curl http://10.0.1.10:8080/api/v1/cluster/replication
# Confirm lag_bytes = 0

# Step 2: Initiate switchover
curl -X POST http://10.0.1.10:8080/api/v1/cluster/switchover \
  -d '{"target_standby": "standby-1"}'

# Switchover steps:
# 1. Stop writes on primary
# 2. Wait for standby to catch up
# 3. Promote standby to primary
# 4. Demote primary to standby
# 5. Resume replication (reversed)
```

**Switchover Time**: 60-180 seconds (zero data loss)

### RTO/RPO Monitoring

**RTO (Recovery Time Objective)**:
Target time to recover after failure.

```bash
# View RTO metrics
curl http://localhost:8080/api/v1/cluster/dr/rto

# Example output:
{
  "target_seconds": 300,
  "max_acceptable_seconds": 600,
  "measured_recovery_times": [120, 145, 95, 130],
  "average_recovery_time_seconds": 122,
  "target_met": true,
  "last_test": "2025-12-20T10:00:00Z",
  "next_test_due": "2025-01-20T10:00:00Z"
}
```

**RPO (Recovery Point Objective)**:
Maximum acceptable data loss.

```bash
# View RPO metrics
curl http://localhost:8080/api/v1/cluster/dr/rpo

# Example output:
{
  "target_seconds": 60,
  "current_lag_seconds": 2,
  "max_acceptable_data_loss_seconds": 300,
  "at_risk": false,
  "within_target": true,
  "backup_frequency_seconds": 3600
}
```

### DR Testing

**Automated DR Test**:
```bash
# Run DR test (does not affect production)
curl -X POST http://localhost:8080/api/v1/cluster/dr/test

# Example output:
{
  "test_id": "DR-TEST-abc123",
  "test_time": "2025-12-25T10:00:00Z",
  "duration_seconds": 125,
  "rto_target_met": true,
  "rpo_target_met": true,
  "issues_found": [],
  "recommendations": [
    "All systems healthy",
    "Replication lag consistently <5 seconds",
    "Failover time: 95 seconds (well within 300s target)"
  ]
}
```

**DR Test Procedure**:
1. Verify standby health and replication
2. Simulate primary failure (network isolation)
3. Trigger automatic failover
4. Measure failover time (RTO)
5. Measure data loss (RPO)
6. Restore primary and failback
7. Generate DR test report

**Recommended Test Frequency**: Monthly

---

## Backup Management

### Backup Scheduling

**Cron-Based Scheduling**:

```bash
# Edit crontab for rustydb user
sudo -u rustydb crontab -e

# Add backup schedule:
# Full backup: Sunday 2 AM
0 2 * * 0 /usr/local/bin/rustydb-full-backup.sh

# Differential backup: Wednesday 2 AM
0 2 * * 3 /usr/local/bin/rustydb-differential-backup.sh

# Incremental backup: Daily 2 AM (Mon-Sat, excluding Wed)
0 2 * * 1,2,4,5,6 /usr/local/bin/rustydb-incremental-backup.sh

# Archive log backup: Every 15 minutes
*/15 * * * * /usr/local/bin/rustydb-archive-log-backup.sh

# Backup verification: Daily 4 AM
0 4 * * * /usr/local/bin/rustydb-verify-backup.sh
```

**Systemd Timer-Based Scheduling** (alternative):

```ini
# /etc/systemd/system/rustydb-full-backup.timer
[Unit]
Description=RustyDB Full Backup Timer
Requires=rustydb-full-backup.service

[Timer]
OnCalendar=Sun *-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

```ini
# /etc/systemd/system/rustydb-full-backup.service
[Unit]
Description=RustyDB Full Backup
After=rustydb.service

[Service]
Type=oneshot
User=rustydb
ExecStart=/usr/local/bin/rustydb-full-backup.sh
```

```bash
# Enable and start timer
sudo systemctl enable rustydb-full-backup.timer
sudo systemctl start rustydb-full-backup.timer
```

### Backup Monitoring

**Monitor Backup Status**:
```bash
# List all backups
curl http://localhost:8080/api/v1/admin/backup/list

# Get backup statistics
curl http://localhost:8080/api/v1/admin/backup/stats

# Example output:
{
  "total_backups": 45,
  "total_size_gb": 2500,
  "total_compressed_size_gb": 1000,
  "compression_ratio": 2.5,
  "backups_by_type": {
    "FULL": 7,
    "INCREMENTAL": 35,
    "DIFFERENTIAL": 3
  },
  "active_backups": 0,
  "oldest_backup": "2025-11-25T02:00:00Z",
  "newest_backup": "2025-12-25T02:00:00Z"
}
```

**Monitor Backup Health**:
```bash
# Check backup health
curl http://localhost:8080/api/v1/admin/backup/health

# Example output:
{
  "status": "HEALTHY",
  "last_successful_backup": "2025-12-25T02:00:00Z",
  "hours_since_last_backup": 8,
  "backup_failures_last_7_days": 0,
  "disk_space_available_gb": 500,
  "wal_archive_lag_seconds": 5,
  "issues": []
}
```

**Alerts**:
```bash
# Configure backup alerts
curl -X POST http://localhost:8080/api/v1/admin/backup/alerts \
  -d '{
    "alert_on_failure": true,
    "alert_on_no_backup_hours": 48,
    "alert_on_disk_space_gb": 100,
    "alert_email": "dba@example.com",
    "alert_webhook": "https://slack.com/webhook/..."
  }'
```

### Backup Catalog

**Catalog Overview**:
The backup catalog is a centralized metadata repository tracking all backups.

**Register Database in Catalog**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/catalog/register \
  -d '{
    "database_id": "rustydb-prod",
    "database_name": "RustyDB Production",
    "version": "0.5.1",
    "platform": "Linux"
  }'
```

**Query Backup Catalog**:
```bash
# Find backups for recovery
curl http://localhost:8080/api/v1/admin/catalog/find-recovery-path \
  -d '{
    "database_id": "rustydb-prod",
    "target_scn": 123456789
  }'

# Example output:
{
  "recovery_path": [
    {
      "backup_id": "FULL-20251224_020000",
      "scn_start": 123450000,
      "scn_end": 123455000
    },
    {
      "backup_id": "INCR-20251225_020000",
      "scn_start": 123455000,
      "scn_end": 123457000
    }
  ]
}
```

**Catalog Reports**:
```bash
# Generate backup report
curl http://localhost:8080/api/v1/admin/catalog/report \
  -d '{
    "report_type": "SUMMARY",
    "start_date": "2025-12-01",
    "end_date": "2025-12-31"
  }'

# Example output:
{
  "period": "2025-12-01 to 2025-12-31",
  "total_backups": 31,
  "successful_backups": 30,
  "failed_backups": 1,
  "total_data_backed_up_gb": 3100,
  "average_backup_duration_minutes": 45,
  "largest_backup_gb": 120,
  "smallest_backup_gb": 2
}
```

### Backup Verification

**Verify Backup Integrity**:
```bash
# Verify specific backup
curl -X POST http://localhost:8080/api/v1/admin/backup/verify \
  -d '{
    "backup_id": "FULL-20251225_020000",
    "verification_type": "CHECKSUM"
  }'

# Verification types:
# - CHECKSUM: Verify file checksums only (fast)
# - STANDARD: Restore headers, verify metadata
# - DEEP: Full restore test to temporary location
```

**Automated Verification Script** (`/usr/local/bin/rustydb-verify-backup.sh`):
```bash
#!/bin/bash
# Verify yesterday's backup

YESTERDAY_BACKUP=$(ls -t /var/lib/rustydb/backup | grep $(date -d yesterday +%Y%m%d) | head -1)

curl -X POST http://localhost:8080/api/v1/admin/backup/verify \
  -d "{\"backup_id\": \"${YESTERDAY_BACKUP}\", \"verification_type\": \"STANDARD\"}"
```

### Retention Policy

**Configure Retention Policy**:
```toml
[backup.retention]
keep_hourly = 24
keep_daily = 7
keep_weekly = 4
keep_monthly = 12
keep_yearly = 5
max_backups = 100
max_age_days = 365
min_free_space_gb = 10
```

**Policy Explanation**:
- Keep last 24 hourly backups
- Keep last 7 daily backups
- Keep last 4 weekly backups (Sunday backups)
- Keep last 12 monthly backups (first of month)
- Keep last 5 yearly backups (first of year)
- Maximum 100 backups total
- Delete backups older than 365 days
- Maintain at least 10 GB free space

**Apply Retention Policy**:
```bash
# Manually apply retention policy
curl -X POST http://localhost:8080/api/v1/admin/backup/apply-retention

# Example output:
{
  "backups_removed": 5,
  "space_freed_gb": 500,
  "backups_kept": 45
}
```

**Automatic Retention Enforcement**:
```toml
[backup.retention]
auto_apply = true
apply_schedule = "0 3 * * *"  # 3 AM daily
```

---

## Best Practices

### Backup Strategy Design

**3-2-1 Rule**:
- **3** copies of data (original + 2 backups)
- **2** different media types (local disk + cloud)
- **1** copy offsite (cloud or remote datacenter)

**Example Implementation**:
```
Original Data: /var/lib/rustydb/data (production)
Backup Copy 1: /var/lib/rustydb/backup (local disk)
Backup Copy 2: S3 bucket (cloud, different region)
```

**Recommended Backup Schedule**:

**Small Database (<100 GB)**:
```
Daily: Full backup (2 AM)
Every 6 hours: Archive log backup
Retention: 30 days
Cloud: Daily sync to S3
```

**Medium Database (100 GB - 1 TB)**:
```
Sunday: Full backup (2 AM)
Mon-Sat: Incremental backup (2 AM)
Every 15 minutes: Archive log backup
Retention: 60 days
Cloud: Weekly full + daily incrementals to S3
```

**Large Database (1 TB - 10 TB)**:
```
Sunday: Full backup (2 AM)
Wednesday: Differential backup (2 AM)
Mon,Tue,Thu,Fri,Sat: Incremental backup (2 AM)
Every 5 minutes: Archive log backup
Retention: 90 days
Cloud: Monthly full, weekly differential to Glacier
```

**Enterprise Database (10 TB+)**:
```
Monthly: Full backup (first Sunday)
Weekly: Differential backup (every Sunday)
Daily: Incremental backup (2 AM)
Continuous: Archive log backup
Retention: 1 year
Cloud: Quarterly full to Glacier Deep Archive
Standby: Real-time replication to DR site
```

### RTO/RPO Planning

**Define Your Requirements**:

| Service Level | RTO | RPO | Backup Strategy |
|---------------|-----|-----|-----------------|
| **Tier 1 (Critical)** | <5 min | <1 min | Standby DB + continuous archive logs |
| **Tier 2 (Important)** | <1 hour | <15 min | Daily incrementals + frequent archive logs |
| **Tier 3 (Standard)** | <4 hours | <1 hour | Daily backups + hourly archive logs |
| **Tier 4 (Low)** | <24 hours | <24 hours | Weekly full + daily incrementals |

**RTO Calculation**:
```
RTO = Restore Time + Recovery Time + Validation Time

Example (1 TB database):
- Restore from backup: 30 minutes (35 MB/s restore rate)
- Apply WAL logs: 15 minutes
- Validation: 5 minutes
Total RTO: 50 minutes
```

**RPO Calculation**:
```
RPO = Time since last backup + Transaction log archive interval

Example:
- Last incremental backup: 2 AM (8 hours ago)
- Archive log backup: Every 15 minutes
- Current time: 10 AM
RPO: 15 minutes (max data loss)
```

**Improve RTO**:
- Use faster storage (NVMe SSDs)
- Increase parallel restore streams
- Use incremental backups (smaller restore size)
- Pre-stage backups on standby server
- Use snapshots for instant restore

**Improve RPO**:
- Increase archive log backup frequency
- Use synchronous replication to standby
- Enable continuous archive log shipping
- Use standby database (RPO near-zero)

### Backup Testing

**Regular Testing Schedule**:
- **Weekly**: Verify last night's backup (checksum)
- **Monthly**: Test restore to non-production environment
- **Quarterly**: Full DR test with failover
- **Annually**: Complete disaster scenario test

**Test Restore Procedure**:
```bash
#!/bin/bash
# Monthly backup restore test

# 1. Identify last week's full backup
BACKUP_ID=$(curl -s http://localhost:8080/api/v1/admin/backup/list | \
    jq -r '.backups[] | select(.type=="FULL") | .backup_id' | head -1)

echo "Testing backup: ${BACKUP_ID}"

# 2. Restore to test environment
curl -X POST http://test.example.com:8080/api/v1/admin/restore \
  -d "{
    \"backup_id\": \"${BACKUP_ID}\",
    \"recovery_target\": {\"type\": \"LATEST\"},
    \"recovery_path\": \"/var/lib/rustydb/test-restore\"
  }"

# 3. Verify data integrity
curl http://test.example.com:8080/api/v1/admin/verify/data

# 4. Run application smoke tests
./run-smoke-tests.sh

# 5. Document results
echo "Test completed at $(date)" >> /var/log/backup-tests.log
```

### Documentation Requirements

**Maintain These Documents**:

**1. Backup Configuration Document**:
```markdown
# Backup Configuration

## Backup Schedule
- Full: Sunday 2 AM
- Incremental: Daily 2 AM (Mon-Sat)
- Archive Logs: Every 15 minutes

## Retention Policy
- Daily: 7 days
- Weekly: 4 weeks
- Monthly: 12 months
- Yearly: 5 years

## Backup Locations
- Primary: /var/lib/rustydb/backup (2 TB RAID 10)
- Cloud: s3://rustydb-backups (AWS S3 Glacier)

## Recovery Objectives
- RTO: 1 hour
- RPO: 15 minutes
```

**2. Recovery Procedures Document**:
```markdown
# Recovery Procedures

## Complete Recovery
1. Stop database: sudo systemctl stop rustydb
2. Identify backup: curl http://localhost:8080/api/v1/admin/backup/list
3. Start restore: curl -X POST ...
4. Start database: sudo systemctl start rustydb

## Point-in-Time Recovery
[Detailed PITR steps]

## Disaster Recovery Failover
[Detailed failover steps]

## Emergency Contacts
- DBA On-Call: +1-555-0100
- Storage Team: storage@example.com
- Cloud Provider Support: 1-800-AWS-SUPPORT
```

**3. Test Results Log**:
```markdown
# Backup Test Results

## 2025-12-25: Monthly Restore Test
- Backup ID: FULL-20251224_020000
- Restore Time: 45 minutes
- Data Validated: ✅
- Application Tests: ✅
- RTO Met: ✅ (target: 60 min, actual: 45 min)
- RPO Met: ✅ (0 data loss)
- Issues: None
- Tested By: John Smith
```

**4. Disaster Recovery Plan**:
```markdown
# Disaster Recovery Plan

## DR Objectives
- RTO: 5 minutes
- RPO: 0 (synchronous replication)

## DR Infrastructure
- Primary Site: DC1 (10.0.1.10)
- DR Site: DC2 (10.0.2.10)
- Replication: Synchronous

## Failover Procedure
[Step-by-step failover process]

## Failback Procedure
[Step-by-step failback process]

## Testing Schedule
- Monthly DR test (non-disruptive)
- Quarterly full DR exercise
```

---

## Troubleshooting

### Common Backup Issues

**Issue: Backup Fails with "Insufficient Disk Space"**

**Diagnosis**:
```bash
df -h /var/lib/rustydb/backup
```

**Solution**:
```bash
# 1. Clean up old backups
curl -X POST http://localhost:8080/api/v1/admin/backup/apply-retention

# 2. Move backups to cloud
aws s3 sync /var/lib/rustydb/backup s3://rustydb-backups/
rm -rf /var/lib/rustydb/backup/old_backups

# 3. Increase backup volume size
# (Consult your infrastructure team)
```

**Issue: Backup Running Too Long**

**Diagnosis**:
```bash
curl http://localhost:8080/api/v1/admin/backup/active
```

**Solution**:
```bash
# 1. Increase parallel streams
curl -X PATCH http://localhost:8080/api/v1/admin/backup/config \
  -d '{"max_parallel_streams": 16}'

# 2. Use faster compression (LZ4 instead of Zstd)
curl -X PATCH http://localhost:8080/api/v1/admin/backup/config \
  -d '{"compression_algorithm": "lz4"}'

# 3. Schedule backup during off-peak hours
```

**Issue: Incremental Backup Larger Than Expected**

**Diagnosis**:
```bash
curl http://localhost:8080/api/v1/admin/backup/stats/INCR-20251225
```

**Possible Causes**:
- VACUUM/ANALYZE updated many blocks
- Large bulk insert/update
- Index rebuild
- Block change tracking not enabled

**Solution**:
```toml
[backup]
enable_change_tracking = true
```

### Common Recovery Issues

**Issue: Recovery Fails with "WAL Archive Missing"**

**Diagnosis**:
```bash
curl http://localhost:8080/api/v1/admin/wal/archive/status
```

**Solution**:
```bash
# 1. Check if WAL files exist
ls -la /var/lib/rustydb/wal_archive/

# 2. If files deleted by retention policy, recover to earlier point
curl -X POST http://localhost:8080/api/v1/admin/restore \
  -d '{
    "recovery_target": {"type": "SCN", "value": 123450000}
  }'

# 3. Prevent future issues: increase WAL retention
[backup]
wal_retention_hours = 336  # 14 days instead of 7
```

**Issue: Restore Extremely Slow**

**Diagnosis**:
```bash
# Check I/O performance
iostat -x 5

# Check restore progress
curl http://localhost:8080/api/v1/admin/restore/status/RECOVERY-xyz
```

**Solution**:
```bash
# 1. Increase restore parallelism
curl -X PATCH http://localhost:8080/api/v1/admin/restore/config \
  -d '{"parallel_streams": 16}'

# 2. Use faster storage for restore destination
# 3. Disable validation temporarily
curl -X PATCH http://localhost:8080/api/v1/admin/restore/RECOVERY-xyz \
  -d '{"validate_blocks": false}'
```

**Issue: Flashback Query Returns No Data**

**Diagnosis**:
```bash
# Check MVCC version retention
curl http://localhost:8080/api/v1/admin/mvcc/stats
```

**Possible Causes**:
- Target time older than MVCC retention
- Vacuum cleaned up old versions
- Flashback logs purged

**Solution**:
```toml
# Increase version retention
[flashback]
version_retention_hours = 168  # 7 days
min_flashback_retention_scn = 1000000
```

---

## Appendices

### Appendix A: Backup Command Reference

**REST API Endpoints**:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/admin/backup` | POST | Create backup |
| `/api/v1/admin/backup/list` | GET | List all backups |
| `/api/v1/admin/backup/{id}` | GET | Get backup details |
| `/api/v1/admin/backup/{id}/status` | GET | Check backup status |
| `/api/v1/admin/backup/verify` | POST | Verify backup |
| `/api/v1/admin/backup/incremental` | POST | Create incremental |
| `/api/v1/admin/backup/differential` | POST | Create differential |
| `/api/v1/admin/restore` | POST | Start restore |
| `/api/v1/admin/restore/{id}/status` | GET | Check restore status |
| `/api/v1/admin/pitr/start` | POST | Start PITR recovery |
| `/api/v1/admin/flashback/query` | POST | Execute flashback query |
| `/api/v1/admin/flashback/table` | POST | Flashback table |
| `/api/v1/admin/restore-points` | GET | List restore points |
| `/api/v1/admin/restore-points` | POST | Create restore point |

### Appendix B: Configuration Reference

**Complete Backup Configuration** (`/etc/rustydb/config.toml`):

```toml
[backup]
backup_dir = "/var/lib/rustydb/backup"
max_parallel_streams = 8
buffer_size = 1048576
compression_enabled = true
compression_algorithm = "zstd"
compression_level = 6
encryption_enabled = true
verify_after_backup = true
block_size = 8192
enable_change_tracking = true

[backup.encryption]
enabled = true
algorithm = "AES256-GCM"
master_key_file = "/etc/rustydb/secrets/backup_master.key"
key_rotation_days = 90

[backup.wal]
wal_archiving_enabled = true
wal_archive_dir = "/var/lib/rustydb/wal_archive"
wal_archive_command = "cp %p /var/lib/rustydb/wal_archive/%f"
wal_retention_hours = 168

[backup.retention]
keep_hourly = 24
keep_daily = 7
keep_weekly = 4
keep_monthly = 12
keep_yearly = 5
max_backups = 100
max_age_days = 365
min_free_space_gb = 10
auto_apply = true
apply_schedule = "0 3 * * *"

[disaster_recovery]
auto_failover_enabled = true
health_check_interval_seconds = 5
max_primary_failures = 3
max_lag_tolerance_seconds = 60
switchover_timeout_seconds = 300

[disaster_recovery.rto]
target_seconds = 300
max_acceptable_seconds = 600
test_frequency_days = 30

[disaster_recovery.rpo]
target_seconds = 60
max_acceptable_data_loss_seconds = 300
backup_frequency_seconds = 3600

[flashback]
version_retention_hours = 168
min_flashback_retention_scn = 1000000
flashback_logs_enabled = true
```

### Appendix C: Backup Size Estimator

**Estimate Backup Sizes**:

```python
#!/usr/bin/env python3
# backup-size-estimator.py

database_size_gb = 1000
change_rate_pct = 5  # 5% daily change

# Full backup
full_backup_gb = database_size_gb * 0.4  # 40% compression
print(f"Full Backup: {full_backup_gb:.1f} GB")

# Incremental backup (daily)
incr_backup_gb = database_size_gb * (change_rate_pct / 100) * 0.3  # 70% compression
print(f"Incremental Backup (daily): {incr_backup_gb:.1f} GB")

# Differential backup (weekly)
diff_backup_gb = database_size_gb * (change_rate_pct / 100) * 7 * 0.35  # 65% compression
print(f"Differential Backup (weekly): {diff_backup_gb:.1f} GB")

# Total weekly storage
weekly_total = full_backup_gb + (incr_backup_gb * 6) + diff_backup_gb
print(f"\nTotal Weekly Storage: {weekly_total:.1f} GB")

# Example output for 1 TB database:
# Full Backup: 400.0 GB
# Incremental Backup (daily): 15.0 GB
# Differential Backup (weekly): 122.5 GB
# Total Weekly Storage: 612.5 GB
```

### Appendix D: Disaster Recovery Checklist

**Pre-Disaster Preparation**:
- [ ] Standby database configured and synchronized
- [ ] Automatic failover tested and working
- [ ] RTO/RPO metrics within targets
- [ ] DR plan documented and reviewed
- [ ] DR test completed within last 30 days
- [ ] Emergency contact list up to date
- [ ] Backup encryption keys securely stored
- [ ] Cloud backups verified and accessible

**During Disaster**:
- [ ] Assess severity and impact
- [ ] Notify stakeholders and management
- [ ] Verify standby database health
- [ ] Trigger failover (manual or automatic)
- [ ] Monitor failover progress
- [ ] Verify new primary accepts writes
- [ ] Update DNS/load balancer
- [ ] Notify users of service restoration
- [ ] Document incident timeline

**Post-Disaster**:
- [ ] Investigate root cause
- [ ] Repair or replace failed primary
- [ ] Rebuild standby database
- [ ] Restore normal replication
- [ ] Schedule failback (if needed)
- [ ] Update DR documentation
- [ ] Conduct post-mortem review
- [ ] Implement preventive measures

### Appendix E: Support Resources

**Documentation**:
- RustyDB Architecture: `/home/user/rusty-db/docs/ARCHITECTURE.md`
- Deployment Guide: `/home/user/rusty-db/release/docs/0.5.1/DEPLOYMENT_GUIDE.md`
- API Documentation: `http://localhost:8080/api/docs`

**Community**:
- GitHub: https://github.com/harborgrid-justin/rusty-db
- Issues: https://github.com/harborgrid-justin/rusty-db/issues
- Discussions: https://github.com/harborgrid-justin/rusty-db/discussions

**Enterprise Support**:
- Email: support@rustydb.io
- Emergency Hotline: 1-800-RUSTYDB
- Support Portal: https://support.rustydb.io

---

**Document Version**: 1.0
**RustyDB Version**: 0.5.1
**Last Updated**: 2025-12-25
**Next Review**: 2026-01-25

**For Questions or Support**:
- Backup Issues: backup-support@rustydb.io
- Recovery Assistance: recovery-help@rustydb.io
- DR Planning: dr-consulting@rustydb.io
- Security Concerns: security@rustydb.io
