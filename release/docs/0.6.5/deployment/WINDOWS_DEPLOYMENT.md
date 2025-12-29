# RustyDB v0.6.5 - Windows Production Deployment Guide

**Version**: 0.6.5
**Release Date**: December 29, 2025
**Status**: ✅ Validated for Windows Enterprise Deployment
**Target**: Windows Server 2019/2022

---

## Executive Summary

Complete deployment guide for RustyDB v0.6.5 on Windows Server environments. Covers installation, Windows Service configuration, security, and operational procedures for enterprise Windows deployments.

**Note**: Linux is the recommended platform for production. Windows deployment is fully supported but Linux offers better performance due to io_uring support.

---

## System Requirements

### Production Server Specifications

**Minimum**:
- **OS**: Windows Server 2019
- **CPU**: 8 cores x86-64 with AVX2
- **RAM**: 32 GB
- **Storage**: 500 GB SSD
- **Network**: 10 Gbps
- **.NET**: Framework 4.8+

**Recommended**:
- **OS**: Windows Server 2022
- **CPU**: 16-32 cores x86-64 with AVX-512
- **RAM**: 128-256 GB
- **Storage**: 2-4 TB NVMe SSD (Storage Spaces)
- **Network**: 25 Gbps
- **.NET**: Framework 4.8+

---

## Installation

### Binary Installation

```powershell
# Run as Administrator

# Create installation directory
New-Item -ItemType Directory -Path "C:\Program Files\RustyDB\0.6.5\bin" -Force

# Copy binaries from build artifacts
Copy-Item "\\build-server\builds\windows\rusty-db-server.exe" `
  -Destination "C:\Program Files\RustyDB\0.6.5\bin\"
Copy-Item "\\build-server\builds\windows\rusty-db-cli.exe" `
  -Destination "C:\Program Files\RustyDB\0.6.5\bin\"

# Create current version symlink (junction)
cmd /c mklink /J "C:\Program Files\RustyDB\current" "C:\Program Files\RustyDB\0.6.5"

# Verify installation
& "C:\Program Files\RustyDB\current\bin\rusty-db-server.exe" --version
# Output: RustyDB v0.6.5

# Check binary size (should be ~40MB for Windows)
Get-Item "C:\Program Files\RustyDB\current\bin\rusty-db-server.exe" | Select Length
```

### Create Data Directories

```powershell
# Create directory structure
$dirs = @(
    "C:\ProgramData\RustyDB\data",
    "C:\ProgramData\RustyDB\wal",
    "C:\ProgramData\RustyDB\archive",
    "C:\ProgramData\RustyDB\backup",
    "C:\ProgramData\RustyDB\logs",
    "C:\ProgramData\RustyDB\certs",
    "C:\ProgramData\RustyDB\keys"
)

foreach ($dir in $dirs) {
    New-Item -ItemType Directory -Path $dir -Force
}

# Set NTFS permissions (restrict to Administrators and SYSTEM)
icacls "C:\ProgramData\RustyDB\data" /inheritance:r
icacls "C:\ProgramData\RustyDB\data" /grant:r "Administrators:(OI)(CI)F"
icacls "C:\ProgramData\RustyDB\data" /grant:r "SYSTEM:(OI)(CI)F"

icacls "C:\ProgramData\RustyDB\keys" /inheritance:r
icacls "C:\ProgramData\RustyDB\keys" /grant:r "Administrators:(OI)(CI)F"
```

---

## Configuration

### Production Configuration File

```powershell
# Create configuration file
$config = @"
# RustyDB v0.6.5 Windows Production Configuration
# Last Updated: 2025-12-29

[database]
data_directory = "C:\\ProgramData\\RustyDB\\data"
wal_directory = "C:\\ProgramData\\RustyDB\\wal"
archive_directory = "C:\\ProgramData\\RustyDB\\archive"
temp_directory = "C:\\ProgramData\\RustyDB\\temp"

[storage]
page_size = 4096
buffer_pool_size = 107374182400  # 100 GB
buffer_eviction_policy = "ARC"

[network]
host = "0.0.0.0"
port = 5432
api_port = 8080
max_connections = 1000
tls_enabled = true
tls_cert_path = "C:\\ProgramData\\RustyDB\\certs\\server.crt"
tls_key_path = "C:\\ProgramData\\RustyDB\\certs\\server.key"

[transaction]
wal_stripe_count = 8
checkpoint_interval = 300

[security.audit]
enabled = true
log_path = "C:\\ProgramData\\RustyDB\\logs\\audit.log"

[logging]
level = "info"
output = "C:\\ProgramData\\RustyDB\\logs\\rustydb.log"
"@

$config | Out-File -FilePath "C:\ProgramData\RustyDB\rustydb.toml" -Encoding UTF8

# Set permissions
icacls "C:\ProgramData\RustyDB\rustydb.toml" /grant:r "Administrators:(R)"
```

---

## Windows Service Installation

### Create Service Account

```powershell
# Create dedicated service account
$password = ConvertTo-SecureString "SecureServicePass123!@#" -AsPlainText -Force
New-LocalUser -Name "rustydb_svc" `
  -Password $password `
  -FullName "RustyDB Service Account" `
  -Description "Service account for RustyDB database server" `
  -PasswordNeverExpires

# Grant "Log on as a service" right
# Manual step required:
# 1. Open Local Security Policy (secpol.msc)
# 2. Navigate to: Local Policies > User Rights Assignment
# 3. Open "Log on as a service"
# 4. Add "rustydb_svc" user

Write-Host "⚠️ Manual step required: Grant 'Log on as a service' right to rustydb_svc"
Write-Host "1. Run: secpol.msc"
Write-Host "2. Navigate to: Local Policies > User Rights Assignment"
Write-Host "3. Edit 'Log on as a service'"
Write-Host "4. Add user: rustydb_svc"
Read-Host "Press Enter after completing manual step"

# Grant permissions to data directories
icacls "C:\ProgramData\RustyDB" /grant "rustydb_svc:(OI)(CI)F" /T
```

### Install Windows Service

```batch
REM Run Command Prompt as Administrator

REM Install service
sc create RustyDB ^
  binPath= "\"C:\Program Files\RustyDB\current\bin\rusty-db-server.exe\" --config \"C:\ProgramData\RustyDB\rustydb.toml\"" ^
  DisplayName= "RustyDB v0.6.5 Enterprise Database" ^
  start= auto ^
  obj= ".\rustydb_svc" ^
  password= "SecureServicePass123!@#"

REM Configure service description
sc description RustyDB "RustyDB v0.6.5 - Enterprise-grade database management system for Windows Server"

REM Configure service recovery actions
sc failure RustyDB reset= 86400 actions= restart/60000/restart/120000/restart/300000

REM Set service to delayed auto-start (better for boot performance)
sc config RustyDB start= delayed-auto

REM Verify service installation
sc query RustyDB
```

### Configure Service Dependencies

```powershell
# Ensure network is available before starting
sc config RustyDB depend= Tcpip/Afd
```

---

## Security Hardening

### 1. Generate TLS Certificates

```powershell
# Using OpenSSL for Windows (install from: https://slproweb.com/products/Win32OpenSSL.html)
cd C:\ProgramData\RustyDB\certs

# Generate CA certificate
& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" genrsa -out ca.key 4096
& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" req -new -x509 -days 3650 -key ca.key -out ca.crt `
  -subj "/C=US/ST=CA/O=Enterprise/CN=RustyDB Windows CA"

# Generate server certificate
& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" genrsa -out server.key 4096
& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" req -new -key server.key -out server.csr `
  -subj "/C=US/ST=CA/O=Enterprise/CN=rustydb.prod.example.com"

# Sign server certificate
& "C:\Program Files\OpenSSL-Win64\bin\openssl.exe" x509 -req -days 365 -in server.csr `
  -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt

# Set NTFS permissions
icacls "*.key" /grant:r "rustydb_svc:(R)"
icacls "*.crt" /grant:r "rustydb_svc:(R)"
```

### 2. Configure Windows Firewall

```powershell
# Create firewall rules
New-NetFirewallRule -DisplayName "RustyDB Server" `
  -Direction Inbound -Protocol TCP -LocalPort 5432 `
  -Action Allow -Profile Domain,Private

New-NetFirewallRule -DisplayName "RustyDB REST API" `
  -Direction Inbound -Protocol TCP -LocalPort 8080 `
  -Action Allow -Profile Domain,Private

New-NetFirewallRule -DisplayName "RustyDB Metrics" `
  -Direction Inbound -Protocol TCP -LocalPort 9090 `
  -Action Allow -Profile Domain,Private

# Verify rules
Get-NetFirewallRule -DisplayName "RustyDB*" | Select-Object DisplayName, Enabled, Direction, Action
```

### 3. Configure Windows Defender

```powershell
# Add exclusions for RustyDB directories (to improve performance)
Add-MpPreference -ExclusionPath "C:\Program Files\RustyDB"
Add-MpPreference -ExclusionPath "C:\ProgramData\RustyDB\data"
Add-MpPreference -ExclusionPath "C:\ProgramData\RustyDB\wal"

# Add exclusion for server process
Add-MpPreference -ExclusionProcess "rusty-db-server.exe"

# Verify exclusions
Get-MpPreference | Select-Object ExclusionPath, ExclusionProcess
```

---

## Service Management

### Start/Stop Service

```powershell
# Start service
Start-Service -Name RustyDB

# Stop service
Stop-Service -Name RustyDB

# Restart service
Restart-Service -Name RustyDB

# Check status
Get-Service -Name RustyDB

# View service details
sc query RustyDB
```

### View Logs

**Windows Event Log**:
```powershell
# View RustyDB events
Get-EventLog -LogName Application -Source RustyDB -Newest 50

# View errors only
Get-EventLog -LogName Application -Source RustyDB -EntryType Error -Newest 20

# Real-time monitoring
Get-EventLog -LogName Application -Source RustyDB -Newest 1 -After (Get-Date).AddMinutes(-1) | Format-List
```

**Application Logs**:
```powershell
# View application log
Get-Content "C:\ProgramData\RustyDB\logs\rustydb.log" -Tail 50 -Wait

# View audit log
Get-Content "C:\ProgramData\RustyDB\logs\audit.log" -Tail 50 -Wait
```

---

## Database Initialization

```powershell
# Initialize database
& "C:\Program Files\RustyDB\current\bin\rusty-db-server.exe" --init --config "C:\ProgramData\RustyDB\rustydb.toml"

# Expected output:
# Initializing RustyDB v0.6.5 database cluster
# Data directory: C:\ProgramData\RustyDB\data
# Initialization complete
```

---

## Backup Configuration

### Automated Backup Script

```powershell
# Create backup script
$backupScript = @'
# RustyDB Automated Backup Script
param(
    [string]$BackupType = "incremental"
)

$BackupDir = "C:\ProgramData\RustyDB\backup"
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$RetentionDays = 7

# Full backup on Sunday, incremental otherwise
if ((Get-Date).DayOfWeek -eq "Sunday") {
    $BackupType = "full"
}

Write-Host "[$(Get-Date)] Starting $BackupType backup"

# Execute backup
& "C:\Program Files\RustyDB\current\bin\rusty-db-cli.exe" backup $BackupType `
    --output "$BackupDir\${BackupType}_backup_$Timestamp.tar.gz" `
    --compress gzip `
    --verify

# Upload to cloud (if configured)
if (Get-Command az -ErrorAction SilentlyContinue) {
    az storage blob upload `
        --account-name "companybackups" `
        --container-name "rustydb" `
        --file "$BackupDir\${BackupType}_backup_$Timestamp.tar.gz" `
        --name "${BackupType}_backup_$Timestamp.tar.gz"
}

# Clean old backups
Get-ChildItem -Path $BackupDir -Filter "*.tar.gz" |
    Where-Object {$_.LastWriteTime -lt (Get-Date).AddDays(-$RetentionDays)} |
    Remove-Item -Force

Write-Host "[$(Get-Date)] Backup completed: ${BackupType}_backup_$Timestamp.tar.gz"
'@

$backupScript | Out-File -FilePath "C:\Program Files\RustyDB\scripts\backup.ps1" -Encoding UTF8
```

### Schedule Backup Task

```powershell
# Create scheduled task for daily backup at 2 AM
$action = New-ScheduledTaskAction -Execute "PowerShell.exe" `
  -Argument "-File `"C:\Program Files\RustyDB\scripts\backup.ps1`""

$trigger = New-ScheduledTaskTrigger -Daily -At 2am

$principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -RunLevel Highest

$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -RestartCount 3

Register-ScheduledTask -TaskName "RustyDB Daily Backup" `
  -Action $action `
  -Trigger $trigger `
  -Principal $principal `
  -Settings $settings `
  -Description "Automated daily backup of RustyDB database"
```

---

## Monitoring Integration

### Windows Performance Counters

```powershell
# RustyDB exposes performance counters under:
# \RustyDB\Transactions Per Second
# \RustyDB\Buffer Pool Hit Rate
# \RustyDB\Active Connections

# Monitor performance counters
Get-Counter "\RustyDB\*"

# Continuous monitoring
Get-Counter "\RustyDB\*" -Continuous
```

### Prometheus Integration

```powershell
# Install Prometheus as Windows service
# Download from: https://prometheus.io/download/

# Configure Prometheus to scrape RustyDB
$prometheusConfig = @"
scrape_configs:
  - job_name: 'rustydb'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          instance: 'rustydb-win-prod-01'
          environment: 'production'
"@

$prometheusConfig | Out-File -FilePath "C:\Prometheus\prometheus.yml" -Encoding UTF8 -Append
```

---

## High Availability (Windows Failover Clustering)

### Configure Windows Server Failover Clustering

```powershell
# Install Failover Clustering feature
Install-WindowsFeature -Name Failover-Clustering -IncludeManagementTools

# Create cluster
New-Cluster -Name "RustyDB-Cluster" `
  -Node "Server1", "Server2", "Server3" `
  -StaticAddress "10.0.2.100"

# Add RustyDB as clustered service
Add-ClusterGenericServiceRole -ServiceName "RustyDB" `
  -Name "RustyDB-Service" `
  -StaticAddress "10.0.2.101"

# Configure shared storage
Add-ClusterSharedVolume -Name "RustyDB-Data"
```

---

## Troubleshooting

### Service Fails to Start

```powershell
# Check Event Viewer
Get-EventLog -LogName Application -Source RustyDB -Newest 10

# Common issues:
# 1. Service account permissions
# 2. Port already in use
# 3. Configuration errors

# Verify service account permissions
icacls "C:\ProgramData\RustyDB\data"

# Check port availability
Get-NetTCPConnection -LocalPort 5432 -State Listen

# Validate configuration
& "C:\Program Files\RustyDB\current\bin\rusty-db-server.exe" --validate-config "C:\ProgramData\RustyDB\rustydb.toml"
```

### Port Already in Use

```powershell
# Find process using port
Get-NetTCPConnection -LocalPort 5432 | Select-Object OwningProcess
Get-Process -Id <process_id>

# Change port in configuration if needed
```

### Performance Issues

```powershell
# Check resource usage
Get-Counter "\Process(rusty-db-server)\% Processor Time"
Get-Counter "\Process(rusty-db-server)\Working Set"
Get-Counter "\Process(rusty-db-server)\IO Data Bytes/sec"

# Enable performance logging
logman create counter RustyDB_Perf -cf counters.txt -si 60 -f csv -o "C:\ProgramData\RustyDB\logs\perf.csv"
logman start RustyDB_Perf
```

---

## Production Checklist

- [ ] Windows Server 2019/2022 installed with latest updates
- [ ] Binary installed (v0.6.5, ~40MB)
- [ ] Data directories created with correct NTFS permissions
- [ ] Configuration file created and validated
- [ ] TLS certificates generated
- [ ] Service account created with proper permissions
- [ ] Windows service installed and configured
- [ ] Firewall rules configured
- [ ] Windows Defender exclusions added
- [ ] Database initialized
- [ ] Backup script created and scheduled
- [ ] Monitoring configured
- [ ] Health check responding
- [ ] Service starts automatically on boot
- [ ] Failover clustering configured (if HA)

---

## Next Steps

1. **Security**: Enable TDE, audit logging, VPD policies
2. **High Availability**: Configure Windows Failover Clustering
3. **Monitoring**: Integrate with SCOM or Prometheus
4. **Application Integration**: Node.js adapter, REST API
5. **Disaster Recovery**: Configure geo-replication

---

**Document Version**: 1.0
**Last Updated**: December 29, 2025
**Status**: ✅ Validated for Windows Enterprise Deployment

---

*RustyDB v0.6.5 - Production Windows Server Deployment*
*$856M Enterprise Database - Windows Server Ready*
