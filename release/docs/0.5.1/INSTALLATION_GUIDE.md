# RustyDB v0.5.1 Installation Guide

**Version**: 0.5.1
**Release Date**: December 25, 2025
**Document Version**: 1.0
**Last Updated**: December 27, 2025

---

## Table of Contents

1. [Introduction](#introduction)
2. [System Requirements](#system-requirements)
3. [Pre-Installation Checklist](#pre-installation-checklist)
4. [Installation Methods](#installation-methods)
5. [Post-Installation Configuration](#post-installation-configuration)
6. [Installation Verification](#installation-verification)
7. [Upgrade Procedures](#upgrade-procedures)
8. [Uninstallation](#uninstallation)
9. [Troubleshooting](#troubleshooting)
10. [Appendices](#appendices)

---

## Introduction

### About This Guide

This Installation Guide provides comprehensive instructions for installing, configuring, and deploying RustyDB v0.5.1 Enterprise Edition in production environments. This document is designed for:

- **Database Administrators (DBAs)** responsible for database deployment
- **System Administrators** managing database infrastructure
- **DevOps Engineers** automating database provisioning
- **IT Architects** planning enterprise deployments

### Document Scope

This guide covers:
- Hardware and software requirements
- Installation procedures for all supported platforms
- Post-installation configuration and tuning
- Upgrade and migration procedures
- Verification and validation procedures
- Troubleshooting common installation issues

### Related Documentation

- **[Quick Start Guide](./QUICK_START.md)** - Rapid deployment for development environments
- **[Deployment Guide](./DEPLOYMENT_GUIDE.md)** - Production deployment strategies
- **[Release Notes](./RELEASE_NOTES.md)** - Version 0.5.1 features and changes
- **[Security Guide](./SECURITY.md)** - Security configuration and hardening
- **[Operations Guide](./OPERATIONS.md)** - Ongoing operations and maintenance

### Conventions Used

This document uses the following conventions:

| Convention | Meaning |
|------------|---------|
| `monospace` | Commands, file names, code snippets |
| **bold** | Important terms, warnings |
| *italic* | References to other sections or documents |
| > Note: | Additional information |
| > Warning: | Critical information requiring attention |
| ✓ | Recommended practice |
| ✗ | Not recommended |

---

## System Requirements

### Hardware Requirements

#### Minimum Requirements (Development/Testing)

| Component | Specification |
|-----------|---------------|
| **CPU** | 2 cores (x86_64 or ARM64) |
| **RAM** | 2 GB |
| **Disk Space** | 5 GB (system + data) |
| **Network** | 100 Mbps |
| **Architecture** | 64-bit only |

#### Recommended Requirements (Production)

| Component | Specification |
|-----------|---------------|
| **CPU** | 8+ cores (3.0 GHz+) |
| **RAM** | 16 GB minimum, 64 GB+ recommended |
| **Disk Space** | 100 GB+ SSD/NVMe (dedicated partition) |
| **Network** | 1 Gbps+ (10 Gbps for clustering) |
| **Architecture** | x86_64 with AVX2 support |

#### Enterprise Production Requirements

| Component | Specification |
|-----------|---------------|
| **CPU** | 16+ cores (3.5 GHz+), Intel Xeon or AMD EPYC |
| **RAM** | 128 GB+ (512 GB for large databases) |
| **Disk Space** | 1 TB+ enterprise SSD/NVMe (RAID 10 recommended) |
| **Network** | Dual 10 Gbps NICs (bonded for redundancy) |
| **Redundancy** | Dual power supplies, ECC RAM |
| **Storage** | Dedicated storage subsystem (SAN/NAS for HA setups) |

#### Disk Space Requirements by Component

| Component | Space Required | Notes |
|-----------|----------------|-------|
| Binaries | 50-100 MB | Server and CLI executables |
| Data Directory | Variable | Depends on database size (plan 3x data size) |
| WAL Logs | 10-50 GB | Write-Ahead Log segments (configurable) |
| Backup Storage | 2x data size | For full and incremental backups |
| Log Files | 5-20 GB | Application and audit logs |
| Temporary Files | 10-50 GB | Sort operations, temp tables |

### Operating System Requirements

#### Supported Operating Systems

##### Linux (Primary Platform - Recommended)

| Distribution | Version | Kernel | Architecture |
|--------------|---------|--------|--------------|
| **Ubuntu** | 20.04 LTS, 22.04 LTS, 24.04 LTS | 5.4+ | x86_64, ARM64 |
| **RHEL** | 8.x, 9.x | 4.18+ | x86_64, ARM64 |
| **CentOS** | 8, 9 (Stream) | 4.18+ | x86_64, ARM64 |
| **Rocky Linux** | 8.x, 9.x | 4.18+ | x86_64, ARM64 |
| **AlmaLinux** | 8.x, 9.x | 4.18+ | x86_64, ARM64 |
| **Debian** | 11 (Bullseye), 12 (Bookworm) | 5.10+ | x86_64, ARM64 |
| **SUSE Linux Enterprise** | 15 SP3+ | 5.3+ | x86_64, ARM64 |
| **Amazon Linux** | 2, 2023 | 4.14+ | x86_64, ARM64 |

##### Windows

| Edition | Version | Architecture |
|---------|---------|--------------|
| **Windows Server** | 2019, 2022 | x86_64 |
| **Windows 10** | Version 1909+ | x86_64 |
| **Windows 11** | All versions | x86_64 |

> Note: Windows support is provided via MinGW-w64 cross-compilation. Linux is recommended for production.

##### macOS

| Version | Architecture | Notes |
|---------|--------------|-------|
| **macOS 11** (Big Sur) | x86_64, ARM64 (M1/M2) | Build from source |
| **macOS 12** (Monterey) | x86_64, ARM64 (M1/M2) | Build from source |
| **macOS 13** (Ventura) | x86_64, ARM64 (M1/M2/M3) | Build from source |
| **macOS 14** (Sonoma) | ARM64 (M1/M2/M3) | Build from source |

> Note: Pre-built macOS binaries not provided. Compile from source using cargo.

#### Linux Kernel Requirements

**Minimum Kernel Version**: 3.2.0
**Recommended Kernel Version**: 5.4+

**Required Kernel Features**:
- Transparent Huge Pages (THP) support
- NUMA awareness (for multi-socket systems)
- `io_uring` support (kernel 5.1+, optional but recommended)
- `cgroups` v2 (for resource management)
- AIO (Asynchronous I/O) support
- File system with direct I/O support

**Kernel Parameters** (tuning required, see Pre-Installation Checklist):
- `vm.swappiness`
- `vm.dirty_ratio`
- `vm.dirty_background_ratio`
- `kernel.shmmax`, `kernel.shmall`
- `net.core.*` (for high-performance networking)

### Software Dependencies

#### Required Software (Linux)

| Software | Version | Purpose |
|----------|---------|---------|
| **glibc** | 2.31+ | C standard library |
| **OpenSSL** | 1.1.1+ or 3.0+ | TLS/SSL support (optional, can be statically linked) |

#### Required Software (Building from Source)

| Software | Version | Installation |
|----------|---------|--------------|
| **Rust** | 1.70+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Cargo** | 1.70+ | Included with Rust |
| **Git** | 2.x | Package manager or git-scm.com |
| **GCC/Clang** | 7.0+ / 10.0+ | `build-essential` (Debian), `gcc` (RHEL) |
| **Make** | 4.0+ | Package manager |

#### Optional Software

| Software | Version | Purpose |
|----------|---------|---------|
| **PostgreSQL Client** | 12+ | Wire protocol compatibility testing |
| **Docker** | 20.10+ | Container deployment |
| **systemd** | 230+ | Service management (Linux) |
| **logrotate** | 3.x | Log file rotation |
| **Prometheus** | 2.x | Metrics collection |
| **Grafana** | 8.x+ | Monitoring dashboards |

### Network Requirements

#### Port Requirements

| Port | Protocol | Purpose | Required | Firewall Rule |
|------|----------|---------|----------|---------------|
| **5432** | TCP | Database server (PostgreSQL wire protocol) | Yes | Allow inbound |
| **8080** | TCP | GraphQL API / REST endpoints | Yes | Allow inbound |
| **9100** | TCP | Prometheus metrics endpoint | No | Allow from monitoring |
| **54321** | TCP | Alternative database port (configurable) | No | As configured |

#### Firewall Configuration Examples

**Linux (ufw)**:
```bash
sudo ufw allow 5432/tcp comment 'RustyDB Server'
sudo ufw allow 8080/tcp comment 'RustyDB GraphQL API'
sudo ufw allow from 10.0.1.0/24 to any port 9100 proto tcp comment 'Metrics (internal)'
```

**Linux (firewalld)**:
```bash
sudo firewall-cmd --permanent --add-port=5432/tcp
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="10.0.1.0/24" port port="9100" protocol="tcp" accept'
sudo firewall-cmd --reload
```

**Windows Firewall**:
```powershell
New-NetFirewallRule -DisplayName "RustyDB Server" -Direction Inbound -LocalPort 5432 -Protocol TCP -Action Allow
New-NetFirewallRule -DisplayName "RustyDB API" -Direction Inbound -LocalPort 8080 -Protocol TCP -Action Allow
```

#### Network Bandwidth Recommendations

| Deployment Type | Bandwidth | Latency |
|-----------------|-----------|---------|
| Development | 100 Mbps | Any |
| Single-instance Production | 1 Gbps | < 10ms (client to server) |
| Clustered HA | 10 Gbps | < 1ms (inter-node) |
| Geo-replicated | 1 Gbps+ | < 100ms (cross-region) |

---

## Pre-Installation Checklist

### System Preparation

#### 1. User and Group Creation

Create a dedicated operating system user and group for RustyDB:

**Linux**:
```bash
# Create rustydb group
sudo groupadd -r rustydb

# Create rustydb user (no login shell, system account)
sudo useradd -r -g rustydb -s /bin/false -d /var/lib/rustydb \
  -c "RustyDB Database Server" rustydb

# Verify creation
id rustydb
# uid=998(rustydb) gid=998(rustydb) groups=998(rustydb)
```

**Windows**:
```powershell
# Create service account (use Computer Management or PowerShell)
New-LocalUser -Name "rustydb" -Description "RustyDB Service Account" `
  -NoPassword -UserMayNotChangePassword -PasswordNeverExpires
```

> Note: For production systems, use a domain service account with appropriate permissions.

#### 2. Directory Structure Creation

Create the standard RustyDB directory hierarchy:

**Linux (FHS-compliant)**:
```bash
# Create base directories
sudo mkdir -p /opt/rustydb/bin
sudo mkdir -p /var/lib/rustydb/data
sudo mkdir -p /var/lib/rustydb/wal
sudo mkdir -p /var/lib/rustydb/backup
sudo mkdir -p /var/log/rustydb
sudo mkdir -p /etc/rustydb/conf
sudo mkdir -p /etc/rustydb/secrets
sudo mkdir -p /run/rustydb
sudo mkdir -p /var/cache/rustydb
sudo mkdir -p /var/tmp/rustydb

# Set ownership
sudo chown -R rustydb:rustydb /opt/rustydb
sudo chown -R rustydb:rustydb /var/lib/rustydb
sudo chown -R rustydb:rustydb /var/log/rustydb
sudo chown -R rustydb:rustydb /etc/rustydb
sudo chown -R rustydb:rustydb /run/rustydb
sudo chown -R rustydb:rustydb /var/cache/rustydb
sudo chown -R rustydb:rustydb /var/tmp/rustydb

# Set permissions (secure defaults)
sudo chmod 750 /opt/rustydb
sudo chmod 700 /var/lib/rustydb
sudo chmod 700 /var/lib/rustydb/data
sudo chmod 700 /var/lib/rustydb/wal
sudo chmod 700 /var/lib/rustydb/backup
sudo chmod 755 /var/log/rustydb
sudo chmod 750 /etc/rustydb
sudo chmod 700 /etc/rustydb/secrets
sudo chmod 755 /run/rustydb
sudo chmod 755 /var/cache/rustydb
sudo chmod 1777 /var/tmp/rustydb  # Sticky bit for tmp
```

**Windows**:
```powershell
# Create directories
New-Item -ItemType Directory -Path "C:\Program Files\RustyDB\bin" -Force
New-Item -ItemType Directory -Path "C:\ProgramData\RustyDB\data" -Force
New-Item -ItemType Directory -Path "C:\ProgramData\RustyDB\logs" -Force
New-Item -ItemType Directory -Path "C:\ProgramData\RustyDB\conf" -Force
New-Item -ItemType Directory -Path "C:\ProgramData\RustyDB\backup" -Force

# Set ACLs (grant rustydb user full control)
$acl = Get-Acl "C:\ProgramData\RustyDB"
$permission = "rustydb","FullControl","ContainerInherit,ObjectInherit","None","Allow"
$accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule $permission
$acl.SetAccessRule($accessRule)
Set-Acl "C:\ProgramData\RustyDB" $acl
```

#### 3. Kernel Parameter Tuning (Linux)

**Edit `/etc/sysctl.conf` or create `/etc/sysctl.d/99-rustydb.conf`**:

```bash
# RustyDB Kernel Tuning Parameters

# Memory Management
vm.swappiness = 10                    # Reduce swap usage
vm.dirty_ratio = 15                   # Percentage of system memory for dirty pages
vm.dirty_background_ratio = 5         # Background writeback threshold
vm.overcommit_memory = 2              # Don't overcommit memory
vm.overcommit_ratio = 90              # Allow 90% of RAM + swap

# Huge Pages (for large memory systems)
vm.nr_hugepages = 512                 # Reserve 1GB (512 * 2MB pages)
vm.hugetlb_shm_group = 998            # GID of rustydb group

# Shared Memory (adjust based on RAM)
kernel.shmmax = 68719476736           # 64 GB
kernel.shmall = 16777216              # 64 GB / 4096 (page size)
kernel.shmmni = 4096                  # Max shared memory segments

# Semaphores (SEMMSL, SEMMNS, SEMOPM, SEMMNI)
kernel.sem = 500 256000 100 1024

# File System
fs.file-max = 6815744                 # Maximum file handles
fs.aio-max-nr = 1048576               # Async I/O requests

# Network Tuning (for high-throughput workloads)
net.core.rmem_max = 134217728         # Max socket receive buffer (128 MB)
net.core.wmem_max = 134217728         # Max socket send buffer (128 MB)
net.core.rmem_default = 16777216      # Default receive buffer (16 MB)
net.core.wmem_default = 16777216      # Default send buffer (16 MB)
net.core.netdev_max_backlog = 10000   # Network device backlog
net.core.somaxconn = 4096             # Max pending connections

net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.tcp_slow_start_after_idle = 0
net.ipv4.tcp_tw_reuse = 1

# Disable Transparent Huge Pages (THP) for database workloads
# This is typically done via boot parameters or runtime:
# echo never > /sys/kernel/mm/transparent_hugepage/enabled
```

**Apply changes**:
```bash
sudo sysctl -p /etc/sysctl.d/99-rustydb.conf
```

**Disable Transparent Huge Pages (THP)**:
```bash
# Temporary (until reboot)
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/defrag

# Permanent (systemd service)
sudo tee /etc/systemd/system/disable-thp.service > /dev/null <<'EOF'
[Unit]
Description=Disable Transparent Huge Pages (THP)
DefaultDependencies=no
After=sysinit.target local-fs.target
Before=rustydb.service

[Service]
Type=oneshot
ExecStart=/bin/sh -c 'echo never > /sys/kernel/mm/transparent_hugepage/enabled'
ExecStart=/bin/sh -c 'echo never > /sys/kernel/mm/transparent_hugepage/defrag'

[Install]
WantedBy=basic.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable disable-thp.service
sudo systemctl start disable-thp.service
```

#### 4. System Limits Configuration (ulimits)

**Edit `/etc/security/limits.d/99-rustydb.conf`**:

```bash
# RustyDB System Limits

# Open Files (file descriptors)
rustydb soft nofile 65536
rustydb hard nofile 65536

# Max processes/threads
rustydb soft nproc 32768
rustydb hard nproc 32768

# Core dump size (unlimited for debugging, or 0 for production)
rustydb soft core unlimited
rustydb hard core unlimited

# Max locked memory (for huge pages)
rustydb soft memlock unlimited
rustydb hard memlock unlimited

# Stack size
rustydb soft stack 10240
rustydb hard stack 32768

# Max file size
rustydb soft fsize unlimited
rustydb hard fsize unlimited
```

**Verify limits** (after logging in as rustydb user):
```bash
sudo -u rustydb bash -c 'ulimit -a'
```

#### 5. Filesystem Recommendations

**Recommended Filesystems**:
- **Linux**: ext4, XFS (preferred for large databases), btrfs
- **Windows**: NTFS with compression disabled
- **macOS**: APFS

**Mount Options (Linux)**:

**ext4**:
```bash
# /etc/fstab entry example
/dev/sdb1  /var/lib/rustydb  ext4  noatime,nodiratime,data=writeback,barrier=0,errors=remount-ro  0  2
```

**XFS** (recommended):
```bash
# /etc/fstab entry example
/dev/sdb1  /var/lib/rustydb  xfs  noatime,nodiratime,logbufs=8,logbsize=256k,swalloc  0  2
```

**Format XFS partition**:
```bash
sudo mkfs.xfs -f -L rustydb_data /dev/sdb1
sudo mount /dev/sdb1 /var/lib/rustydb
```

> Warning: Use `data=writeback` and `barrier=0` only with battery-backed write cache or UPS.

#### 6. Storage Preparation

**RAID Configuration**:
- **RAID 10**: Recommended for database data (best performance + redundancy)
- **RAID 5/6**: Acceptable for read-heavy workloads, slower writes
- **RAID 0**: Not recommended (no redundancy)
- **RAID 1**: Acceptable for small databases

**SSD/NVMe Optimization**:
```bash
# Enable TRIM (for SSDs)
sudo systemctl enable fstrim.timer
sudo systemctl start fstrim.timer

# Verify TRIM support
sudo fstrim -v /var/lib/rustydb
```

**I/O Scheduler Tuning**:
```bash
# For SSDs/NVMe: use none or mq-deadline
echo none | sudo tee /sys/block/nvme0n1/queue/scheduler

# For HDDs: use deadline or mq-deadline
echo mq-deadline | sudo tee /sys/block/sda/queue/scheduler

# Make permanent via udev rule
sudo tee /etc/udev/rules.d/60-rustydb-io-scheduler.rules > /dev/null <<'EOF'
# Set I/O scheduler for NVMe (none)
ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/scheduler}="none"

# Set I/O scheduler for SSDs (mq-deadline)
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="mq-deadline"

# Set I/O scheduler for HDDs (mq-deadline)
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="1", ATTR{queue/scheduler}="mq-deadline"
EOF

sudo udevadm control --reload-rules
```

#### 7. Clock Synchronization

RustyDB requires accurate system time for distributed operations.

**Install and configure NTP (Linux)**:
```bash
# Ubuntu/Debian
sudo apt-get install -y chrony
sudo systemctl enable chronyd
sudo systemctl start chronyd

# RHEL/CentOS
sudo yum install -y chrony
sudo systemctl enable chronyd
sudo systemctl start chronyd

# Verify synchronization
chronyc tracking
```

**Windows**:
```powershell
# Configure Windows Time Service
w32tm /config /manualpeerlist:"time.windows.com,0x8" /syncfromflags:manual /reliable:YES /update
net stop w32time
net start w32time
w32tm /resync
```

#### 8. Security Hardening

**SELinux Configuration (RHEL/CentOS)**:
```bash
# Option 1: Create custom SELinux policy (recommended)
# (Policy creation is complex, consult security team)

# Option 2: Set permissive mode for rustydb (temporary workaround)
sudo semanage permissive -a rustydb_t

# Option 3: Disable SELinux (not recommended for production)
# Edit /etc/selinux/config: SELINUX=disabled
```

**AppArmor Configuration (Ubuntu/Debian)**:
```bash
# Create AppArmor profile for RustyDB
# (Consult AppArmor documentation or use complain mode initially)
sudo aa-complain /opt/rustydb/bin/rusty-db-server
```

### Pre-Installation Verification

Run this verification script before proceeding:

```bash
#!/bin/bash
# rustydb-preinstall-check.sh

echo "RustyDB v0.5.1 Pre-Installation Check"
echo "======================================"
echo ""

# Check OS
echo "[OS] Checking operating system..."
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    echo "✓ OS: $NAME $VERSION"
else
    echo "✗ Cannot detect OS version"
fi
echo ""

# Check kernel
echo "[Kernel] Checking kernel version..."
KERNEL=$(uname -r)
echo "✓ Kernel: $KERNEL"
echo ""

# Check user
echo "[User] Checking rustydb user..."
if id rustydb &>/dev/null; then
    echo "✓ User 'rustydb' exists"
else
    echo "✗ User 'rustydb' does not exist"
fi
echo ""

# Check directories
echo "[Directories] Checking directory structure..."
for dir in /opt/rustydb /var/lib/rustydb /var/log/rustydb /etc/rustydb; do
    if [[ -d $dir ]]; then
        echo "✓ $dir exists"
    else
        echo "✗ $dir does not exist"
    fi
done
echo ""

# Check disk space
echo "[Disk] Checking disk space..."
df -h /var/lib/rustydb 2>/dev/null || df -h /
echo ""

# Check memory
echo "[Memory] Checking available memory..."
free -h
echo ""

# Check ports
echo "[Network] Checking port availability..."
for port in 5432 8080; do
    if sudo lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo "✗ Port $port is in use"
    else
        echo "✓ Port $port is available"
    fi
done
echo ""

# Check kernel parameters
echo "[Kernel] Checking kernel parameters..."
echo "vm.swappiness = $(sysctl -n vm.swappiness)"
echo "vm.dirty_ratio = $(sysctl -n vm.dirty_ratio)"
echo "fs.file-max = $(sysctl -n fs.file-max)"
echo ""

# Check ulimits
echo "[Limits] Checking system limits for rustydb user..."
sudo -u rustydb bash -c 'echo "nofile (open files): $(ulimit -n)"'
sudo -u rustydb bash -c 'echo "nproc (processes): $(ulimit -u)"'
echo ""

echo "Pre-installation check complete."
```

---

## Installation Methods

### Method 1: Installation from Pre-Built Binaries (Recommended)

This is the fastest installation method and recommended for most users.

#### Step 1: Download Binaries

**Linux (x86_64)**:
```bash
# Navigate to builds directory (if you have the repository)
cd /path/to/rusty-db/builds/linux/

# Or download from release artifacts (if available)
# wget https://github.com/harborgrid-justin/rusty-db/releases/download/v0.5.1/rustydb-v0.5.1-linux-x86_64.tar.gz
# tar -xzf rustydb-v0.5.1-linux-x86_64.tar.gz
```

**Windows (x86_64)**:
```powershell
# Navigate to builds directory (if you have the repository)
cd \path\to\rusty-db\builds\windows\

# Or download from release artifacts (if available)
# Invoke-WebRequest -Uri "https://github.com/harborgrid-justin/rusty-db/releases/download/v0.5.1/rustydb-v0.5.1-windows-x86_64.zip" -OutFile "rustydb-v0.5.1-windows-x86_64.zip"
# Expand-Archive -Path rustydb-v0.5.1-windows-x86_64.zip -DestinationPath .
```

#### Step 2: Verify Binary Integrity

**Check file sizes**:
```bash
# Linux
ls -lh rusty-db-server rusty-db-cli
# Expected: rusty-db-server: ~38 MB, rusty-db-cli: ~922 KB

# Windows
dir rusty-db-server.exe rusty-db-cli.exe
# Expected: rusty-db-server.exe: ~41 MB, rusty-db-cli.exe: ~876 KB
```

**Verify binary type** (Linux):
```bash
file rusty-db-server
# Expected: ELF 64-bit LSB executable, x86-64, dynamically linked

ldd rusty-db-server
# Check dependencies (glibc, etc.)
```

#### Step 3: Install Binaries

**Linux**:
```bash
# Make binaries executable
chmod +x rusty-db-server rusty-db-cli

# Copy to installation directory
sudo cp rusty-db-server /opt/rustydb/bin/
sudo cp rusty-db-cli /opt/rustydb/bin/

# Verify ownership
sudo chown rustydb:rustydb /opt/rustydb/bin/*

# Create symlinks for convenience (optional)
sudo ln -s /opt/rustydb/bin/rusty-db-server /usr/local/bin/rusty-db-server
sudo ln -s /opt/rustydb/bin/rusty-db-cli /usr/local/bin/rusty-db-cli

# Verify installation
/opt/rustydb/bin/rusty-db-server --version
```

**Windows**:
```powershell
# Copy to installation directory
Copy-Item rusty-db-server.exe "C:\Program Files\RustyDB\bin\"
Copy-Item rusty-db-cli.exe "C:\Program Files\RustyDB\bin\"

# Add to PATH (system-wide)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\RustyDB\bin", "Machine")

# Verify installation
& "C:\Program Files\RustyDB\bin\rusty-db-server.exe" --version
```

**Expected Output**:
```
RustyDB v0.5.1 - Enterprise Edition
Build: release
Platform: x86_64-unknown-linux-gnu
Rust: 1.92.0
Build Date: 2025-12-25
```

### Method 2: Installation from Source

Building from source provides maximum flexibility and optimization for your specific hardware.

#### Step 1: Install Build Prerequisites

**Linux (Debian/Ubuntu)**:
```bash
sudo apt-get update
sudo apt-get install -y build-essential curl git libssl-dev pkg-config
```

**Linux (RHEL/CentOS)**:
```bash
sudo yum groupinstall -y 'Development Tools'
sudo yum install -y openssl-devel curl git
```

**macOS**:
```bash
xcode-select --install
brew install openssl pkg-config
```

**Windows**:
- Install Visual Studio Build Tools 2019+
- Or install MinGW-w64

#### Step 2: Install Rust Toolchain

```bash
# Install rustup (Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Select default installation (option 1)
# Source the environment
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version

# Expected: rustc 1.70.0 or higher
```

#### Step 3: Clone Repository

```bash
# Clone the repository
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db

# Checkout v0.5.1 release tag
git checkout v0.5.1

# Or checkout specific branch
# git checkout claude/enterprise-docs-generation-8RDSa

# Verify you're on correct version
git describe --tags
```

#### Step 4: Build the Project

**Standard Release Build**:
```bash
# Build release binaries (optimized)
cargo build --release

# Build time: 5-15 minutes (first build)
# Subsequent builds: 1-3 minutes (incremental)
```

**Build with SIMD Optimizations** (Recommended for x86_64):
```bash
# Build with SIMD features enabled
cargo build --release --features simd

# Requires CPU with AVX2 support
```

**Build with Platform-Specific I/O**:
```bash
# Linux with io_uring support (kernel 5.1+)
cargo build --release --features io_uring

# Windows with IOCP support
cargo build --release --features iocp
```

**Cross-Compilation** (Advanced):
```bash
# Install cross-compilation target
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-unknown-linux-gnu

# Cross-compile for Windows (from Linux)
cargo build --release --target x86_64-pc-windows-gnu

# Cross-compile for ARM64
cargo build --release --target aarch64-unknown-linux-gnu
```

#### Step 5: Verify Build

```bash
# Check that binaries were created
ls -lh target/release/rusty-db-server
ls -lh target/release/rusty-db-cli

# Test binary execution
./target/release/rusty-db-server --version
./target/release/rusty-db-cli --version

# Run test suite
cargo test --release

# Expected: All tests pass (1200+ tests)
```

#### Step 6: Install Built Binaries

```bash
# Copy binaries to installation directory (Linux)
sudo cp target/release/rusty-db-server /opt/rustydb/bin/
sudo cp target/release/rusty-db-cli /opt/rustydb/bin/

# Set ownership
sudo chown rustydb:rustydb /opt/rustydb/bin/*

# Set executable permissions
sudo chmod 750 /opt/rustydb/bin/*

# Create symlinks
sudo ln -s /opt/rustydb/bin/rusty-db-server /usr/local/bin/rusty-db-server
sudo ln -s /opt/rustydb/bin/rusty-db-cli /usr/local/bin/rusty-db-cli
```

### Method 3: Docker Container Deployment

Docker provides isolated, portable deployment suitable for development and containerized production environments.

#### Step 1: Create Dockerfile

**Create `Dockerfile` in repository root**:
```dockerfile
# Multi-stage build for minimal image size
FROM rust:1.92-slim-bullseye AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy source code
COPY . .

# Build release binaries
RUN cargo build --release --bins

# ============================================
# Runtime stage
# ============================================
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create rustydb user and group
RUN groupadd -r rustydb && useradd -r -g rustydb -s /bin/false rustydb

# Create directories
RUN mkdir -p /var/lib/rustydb/data \
    /var/lib/rustydb/wal \
    /var/log/rustydb \
    /etc/rustydb \
    && chown -R rustydb:rustydb /var/lib/rustydb /var/log/rustydb /etc/rustydb

# Copy binaries from builder
COPY --from=builder /build/target/release/rusty-db-server /usr/local/bin/
COPY --from=builder /build/target/release/rusty-db-cli /usr/local/bin/

# Copy configuration template
COPY conf/rustydb.toml /etc/rustydb/rustydb.toml
RUN chown rustydb:rustydb /etc/rustydb/rustydb.toml

# Switch to rustydb user
USER rustydb

# Expose ports
EXPOSE 5432 8080 9100

# Set environment variables
ENV RUSTYDB_DATA_DIR=/var/lib/rustydb/data
ENV RUSTYDB_PORT=5432
ENV RUSTYDB_GRAPHQL_PORT=8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD ["/usr/local/bin/rusty-db-cli", "ping"]

# Volume for persistent data
VOLUME ["/var/lib/rustydb", "/var/log/rustydb"]

# Run server
ENTRYPOINT ["/usr/local/bin/rusty-db-server"]
```

#### Step 2: Build Docker Image

```bash
# Build image
docker build -t rustydb:0.5.1 .

# Build with cache optimization
docker build -t rustydb:0.5.1 --build-arg BUILDKIT_INLINE_CACHE=1 .

# Tag image
docker tag rustydb:0.5.1 rustydb:latest

# Verify image
docker images | grep rustydb
```

#### Step 3: Run Docker Container

**Development mode**:
```bash
docker run -d \
  --name rustydb \
  -p 5432:5432 \
  -p 8080:8080 \
  -v rustydb-data:/var/lib/rustydb \
  -v rustydb-logs:/var/log/rustydb \
  rustydb:0.5.1
```

**Production mode with custom configuration**:
```bash
docker run -d \
  --name rustydb-prod \
  -p 5432:5432 \
  -p 8080:8080 \
  -v /opt/rustydb/data:/var/lib/rustydb \
  -v /opt/rustydb/logs:/var/log/rustydb \
  -v /opt/rustydb/conf:/etc/rustydb:ro \
  --restart unless-stopped \
  --memory="16g" \
  --cpus="8" \
  --ulimit nofile=65536:65536 \
  rustydb:0.5.1
```

#### Step 4: Docker Compose Deployment

**Create `docker-compose.yml`**:
```yaml
version: '3.8'

services:
  rustydb:
    image: rustydb:0.5.1
    container_name: rustydb
    restart: unless-stopped

    ports:
      - "5432:5432"
      - "8080:8080"
      - "9100:9100"

    volumes:
      - rustydb-data:/var/lib/rustydb
      - rustydb-logs:/var/log/rustydb
      - ./conf:/etc/rustydb:ro

    environment:
      - RUSTYDB_DATA_DIR=/var/lib/rustydb/data
      - RUSTYDB_PORT=5432
      - RUSTYDB_GRAPHQL_PORT=8080
      - RUSTYDB_BUFFER_POOL_SIZE=10000
      - RUST_LOG=info

    deploy:
      resources:
        limits:
          cpus: '8'
          memory: 16G
        reservations:
          cpus: '4'
          memory: 8G

    healthcheck:
      test: ["CMD", "/usr/local/bin/rusty-db-cli", "ping"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 30s

    networks:
      - rustydb-network

volumes:
  rustydb-data:
    driver: local
  rustydb-logs:
    driver: local

networks:
  rustydb-network:
    driver: bridge
```

**Start with Docker Compose**:
```bash
docker-compose up -d
docker-compose logs -f
```

### Method 4: Package Manager Installation (Future)

Package manager support is planned for future releases.

**Planned Support**:
- **APT** (Debian/Ubuntu): `sudo apt-get install rustydb`
- **YUM/DNF** (RHEL/CentOS): `sudo yum install rustydb`
- **Homebrew** (macOS): `brew install rustydb`
- **Chocolatey** (Windows): `choco install rustydb`

---

## Post-Installation Configuration

### Initial Configuration

#### 1. Configuration File Setup

RustyDB uses TOML-based configuration. A default configuration template is provided.

**Copy configuration template** (Linux):
```bash
# Copy template to configuration directory
sudo cp /opt/rustydb/conf/rustydb.toml /etc/rustydb/rustydb.toml

# Or if installing from repository
sudo cp conf/rustydb.toml /etc/rustydb/rustydb.toml

# Set ownership and permissions
sudo chown rustydb:rustydb /etc/rustydb/rustydb.toml
sudo chmod 640 /etc/rustydb/rustydb.toml
```

**Edit configuration**:
```bash
sudo -u rustydb nano /etc/rustydb/rustydb.toml
```

#### 2. Essential Configuration Parameters

**Minimum required configuration changes**:

```toml
[instance]
name = "production"  # Change from "default"
description = "RustyDB Production Instance"

[paths]
data_dir = "/var/lib/rustydb/data"
logs_dir = "/var/log/rustydb"
backup_dir = "/var/lib/rustydb/backup"

[server]
listen_host = "0.0.0.0"  # Bind to all interfaces (or specific IP)
listen_port = 5432
max_connections = 500    # Adjust based on workload

[security]
mode = "prod"            # Change from "dev"

[tls]
enabled = true           # Enable TLS for production
cert_path = "/etc/rustydb/secrets/tls/server.crt"
key_path = "/etc/rustydb/secrets/tls/server.key"
min_version = "1.3"      # Use TLS 1.3

[auth]
mode = "password"        # Enable authentication (not "none")
session_timeout_ms = 1800000
max_failed_attempts = 5

[logging]
mode = "file"
format = "json"          # Use JSON for log aggregation
level = "info"           # Or "warn" for production
audit_enabled = true

[storage]
page_size = 8192         # 8 KB pages (verified default for v0.5.1)
buffer_pool_pages = 131072  # 1 GB buffer pool (131072 * 8 KB)
fsync = true             # Ensure durability

[wal]
enabled = true
max_segment_mb = 64
checkpoint_interval_ms = 60000
sync_mode = "local"      # Or "remote_write" for synchronous replication
archive_enabled = true   # Enable for PITR
```

#### 3. Environment Variables

Set environment variables for runtime configuration (alternative to config file):

**Linux (systemd)**:
```bash
# Edit systemd service file
sudo systemctl edit rustydb

# Add environment variables in override file:
[Service]
Environment="RUSTYDB_DATA_DIR=/var/lib/rustydb/data"
Environment="RUSTYDB_PORT=5432"
Environment="RUSTYDB_GRAPHQL_PORT=8080"
Environment="RUSTYDB_BUFFER_POOL_SIZE=131072"
Environment="RUST_LOG=info"
```

**Linux (shell)**:
```bash
export RUSTYDB_DATA_DIR=/var/lib/rustydb/data
export RUSTYDB_PORT=5432
export RUSTYDB_GRAPHQL_PORT=8080
export RUSTYDB_BUFFER_POOL_SIZE=131072
export RUST_LOG=info
```

**Windows**:
```powershell
[Environment]::SetEnvironmentVariable("RUSTYDB_DATA_DIR", "C:\ProgramData\RustyDB\data", "Machine")
[Environment]::SetEnvironmentVariable("RUSTYDB_PORT", "5432", "Machine")
[Environment]::SetEnvironmentVariable("RUSTYDB_GRAPHQL_PORT", "8080", "Machine")
```

#### 4. TLS Certificate Configuration

**Generate self-signed certificates** (development only):
```bash
# Create certificates directory
sudo mkdir -p /etc/rustydb/secrets/tls
cd /etc/rustydb/secrets/tls

# Generate private key
sudo openssl genrsa -out server.key 4096

# Generate self-signed certificate (valid for 365 days)
sudo openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=rustydb.example.com"

# Set permissions
sudo chown rustydb:rustydb server.key server.crt
sudo chmod 600 server.key
sudo chmod 644 server.crt
```

**Production certificates**:
- Use certificates from trusted CA (Let's Encrypt, DigiCert, etc.)
- Store private keys securely (HSM, vault, encrypted filesystem)
- Implement certificate rotation procedures

#### 5. Initialize Data Directory

**First-time initialization**:
```bash
# Run server initialization (will create data directory structure)
sudo -u rustydb /opt/rustydb/bin/rusty-db-server --init

# Or start server for the first time (auto-initializes)
sudo -u rustydb /opt/rustydb/bin/rusty-db-server
```

**Verify directory structure**:
```bash
tree -L 2 /var/lib/rustydb/
```

**Expected structure**:
```
/var/lib/rustydb/
├── data/
│   ├── base/          # Table data files
│   ├── meta/          # Metadata
│   └── pg_control     # Control file
├── wal/               # Write-Ahead Log segments
│   └── 000000010000000000000001
├── backup/            # Backup storage
└── cache/             # Query cache
```

#### 6. Service Management Configuration

**systemd Service (Linux)**:

Create `/etc/systemd/system/rustydb.service`:
```ini
[Unit]
Description=RustyDB Enterprise Database Server v0.5.1
Documentation=https://github.com/harborgrid-justin/rusty-db
After=network.target network-online.target
Wants=network-online.target
Requires=disable-thp.service

[Service]
Type=simple
User=rustydb
Group=rustydb
WorkingDirectory=/opt/rustydb

# Binary location
ExecStart=/opt/rustydb/bin/rusty-db-server --config /etc/rustydb/rustydb.toml

# Graceful shutdown
ExecStop=/bin/kill -TERM $MAINPID
TimeoutStopSec=300
KillMode=mixed
KillSignal=SIGTERM

# Restart policy
Restart=on-failure
RestartSec=10s

# Environment
Environment="RUSTYDB_DATA_DIR=/var/lib/rustydb/data"
Environment="RUST_LOG=info"
EnvironmentFile=-/etc/default/rustydb

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustydb

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/rustydb /var/log/rustydb /run/rustydb /var/cache/rustydb
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=false
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6

# Resource limits
LimitNOFILE=65536
LimitNPROC=32768
LimitMEMLOCK=infinity
LimitCORE=infinity

# OOM score adjustment (lower = less likely to be killed)
OOMScoreAdjust=-900

[Install]
WantedBy=multi-user.target
```

**Enable and start service**:
```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service (auto-start on boot)
sudo systemctl enable rustydb

# Start service
sudo systemctl start rustydb

# Check status
sudo systemctl status rustydb

# View logs
sudo journalctl -u rustydb -f

# Stop service
sudo systemctl stop rustydb

# Restart service
sudo systemctl restart rustydb
```

**Windows Service**:

Use NSSM (Non-Sucking Service Manager) or Windows Service wrapper:

```powershell
# Download and install NSSM
# https://nssm.cc/download

# Install RustyDB as Windows service
nssm install RustyDB "C:\Program Files\RustyDB\bin\rusty-db-server.exe"
nssm set RustyDB AppDirectory "C:\ProgramData\RustyDB"
nssm set RustyDB AppEnvironmentExtra "RUSTYDB_DATA_DIR=C:\ProgramData\RustyDB\data"
nssm set RustyDB DisplayName "RustyDB Enterprise Database Server"
nssm set RustyDB Description "RustyDB v0.5.1 Enterprise Database"
nssm set RustyDB Start SERVICE_AUTO_START

# Start service
nssm start RustyDB

# Check status
nssm status RustyDB

# Stop service
nssm stop RustyDB
```

#### 7. Log Rotation Configuration

**logrotate (Linux)**:

Create `/etc/logrotate.d/rustydb`:
```bash
/var/log/rustydb/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0640 rustydb rustydb
    sharedscripts
    postrotate
        /bin/systemctl reload rustydb > /dev/null 2>&1 || true
    endscript
}

/var/log/rustydb/security_audit.log {
    daily
    rotate 90
    compress
    delaycompress
    missingok
    notifempty
    create 0600 rustydb rustydb
    # Don't rotate on size, only on time (compliance requirement)
}
```

**Test log rotation**:
```bash
sudo logrotate -f /etc/logrotate.d/rustydb
```

### Performance Tuning

#### Buffer Pool Sizing

Calculate optimal buffer pool size:
```
Buffer Pool Size = (Available RAM * 0.25 to 0.75) / Page Size

Example for 64 GB RAM, 8 KB pages, 50% RAM allocation:
Buffer Pool Pages = (64 * 1024 * 1024 * 1024 * 0.5) / 8192 = 4,194,304 pages
```

**Configuration**:
```toml
[storage]
page_size = 8192
buffer_pool_pages = 4194304  # 32 GB buffer pool
```

#### Connection Pool Tuning

```toml
[server]
max_connections = 500        # Adjust based on workload
idle_timeout_ms = 300000     # 5 minutes
request_timeout_ms = 30000   # 30 seconds
query_timeout_ms = 600000    # 10 minutes (0 = unlimited)
```

**Calculate max connections**:
```
Max Connections = (Available CPU Cores * 2) to (Available CPU Cores * 4)

Example for 16-core system:
Max Connections = 32 to 64 (conservative)
Max Connections = 200 to 500 (typical web application)
```

---

## Installation Verification

### Basic Verification

#### 1. Version Check

```bash
# Check server version
/opt/rustydb/bin/rusty-db-server --version

# Expected output:
# RustyDB v0.5.1 - Enterprise Edition
# Build: release
# Platform: x86_64-unknown-linux-gnu
```

#### 2. Service Status Check

```bash
# Check service status
sudo systemctl status rustydb

# Expected: Active (running)
```

#### 3. Process Check

```bash
# Check process
ps aux | grep rusty-db-server

# Expected: rustydb user running rusty-db-server
```

#### 4. Port Listening Check

```bash
# Check open ports
sudo netstat -tlnp | grep rusty-db-server
# Or
sudo ss -tlnp | grep rusty-db-server

# Expected output:
# tcp   0   0 0.0.0.0:5432   0.0.0.0:*   LISTEN   12345/rusty-db-server
# tcp   0   0 0.0.0.0:8080   0.0.0.0:*   LISTEN   12345/rusty-db-server
```

### Functional Verification

#### 1. HTTP Health Check

```bash
# Health endpoint
curl http://localhost:8080/health

# Expected response (200 OK):
{
  "status": "healthy",
  "version": "0.5.1",
  "uptime": 120,
  "connections": {
    "active": 0,
    "max": 500
  },
  "buffer_pool": {
    "size": 131072,
    "used": 42,
    "hit_rate": 0.95
  }
}
```

#### 2. GraphQL Endpoint Check

```bash
# Access GraphQL playground
curl http://localhost:8080/graphql

# Or open in browser:
# http://localhost:8080/graphql
```

**Test GraphQL query**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ serverInfo { version uptime activeConnections } }"
  }'

# Expected response:
{
  "data": {
    "serverInfo": {
      "version": "0.5.1",
      "uptime": 300,
      "activeConnections": 1
    }
  }
}
```

#### 3. Transaction Test

**Begin transaction**:
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { beginTransaction(isolationLevel: READ_COMMITTED) { transactionId status timestamp } }"
  }'
```

**Expected response**:
```json
{
  "data": {
    "beginTransaction": {
      "transactionId": "550e8400-e29b-41d4-a716-446655440000",
      "status": "ACTIVE",
      "timestamp": "2025-12-27T12:34:56.789Z"
    }
  }
}
```

#### 4. Metrics Endpoint Check

```bash
# Prometheus metrics
curl http://localhost:9100/metrics

# Expected: Prometheus-formatted metrics
# rustydb_transactions_total{status="committed"} 0
# rustydb_active_connections 1
# rustydb_buffer_pool_hit_rate 0.95
```

#### 5. Log File Verification

```bash
# Check log files exist
ls -la /var/log/rustydb/

# View recent logs
tail -n 50 /var/log/rustydb/rustydb.log

# Expected log entries:
# {"timestamp":"2025-12-27T12:00:00Z","level":"INFO","message":"RustyDB v0.5.1 starting..."}
# {"timestamp":"2025-12-27T12:00:01Z","level":"INFO","message":"Database server listening on 0.0.0.0:5432"}
```

### Performance Verification

#### 1. Load Test (Optional)

```bash
# Simple concurrent connection test
for i in {1..100}; do
  curl -s http://localhost:8080/health > /dev/null &
done
wait

# Check all requests succeeded
echo "Load test complete"
```

#### 2. Buffer Pool Hit Rate

```bash
# Monitor buffer pool performance
curl -s http://localhost:8080/health | jq '.buffer_pool.hit_rate'

# Expected: > 0.90 (90%) after warm-up period
```

### Security Verification

#### 1. File Permissions Check

```bash
# Check critical file permissions
ls -la /etc/rustydb/secrets/
ls -la /var/lib/rustydb/

# Verify:
# - /etc/rustydb/secrets/: 700 (rustydb:rustydb)
# - /var/lib/rustydb/data/: 700 (rustydb:rustydb)
```

#### 2. TLS Verification (if enabled)

```bash
# Test TLS connection
openssl s_client -connect localhost:5432 -tls1_3

# Verify certificate details
```

#### 3. Authentication Verification (if enabled)

```bash
# Test authentication requirement
curl http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ serverInfo { version } }"}'

# Expected: Authentication error if auth enabled
```

### Comprehensive Verification Script

```bash
#!/bin/bash
# rustydb-verify-installation.sh

echo "RustyDB v0.5.1 Installation Verification"
echo "========================================="
echo ""

SUCCESS=0
FAILED=0

# Function to test
test_check() {
    local test_name="$1"
    local test_command="$2"

    echo -n "[$test_name] Testing... "

    if eval "$test_command" > /dev/null 2>&1; then
        echo "✓ PASS"
        ((SUCCESS++))
        return 0
    else
        echo "✗ FAIL"
        ((FAILED++))
        return 1
    fi
}

# Version check
test_check "Version" "/opt/rustydb/bin/rusty-db-server --version | grep -q '0.5.1'"

# Service status
test_check "Service Status" "systemctl is-active --quiet rustydb"

# Process running
test_check "Process Running" "pgrep -f rusty-db-server"

# Port 5432 listening
test_check "Database Port (5432)" "ss -tln | grep -q ':5432'"

# Port 8080 listening
test_check "API Port (8080)" "ss -tln | grep -q ':8080'"

# Health endpoint
test_check "Health Endpoint" "curl -sf http://localhost:8080/health"

# GraphQL endpoint
test_check "GraphQL Endpoint" "curl -sf http://localhost:8080/graphql"

# Data directory
test_check "Data Directory" "test -d /var/lib/rustydb/data"

# Log directory
test_check "Log Directory" "test -d /var/log/rustydb"

# Log file exists
test_check "Log File" "test -f /var/log/rustydb/rustydb.log"

echo ""
echo "Verification Summary"
echo "===================="
echo "Passed: $SUCCESS"
echo "Failed: $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "✓ Installation verification successful!"
    exit 0
else
    echo "✗ Installation verification failed. Please review failed checks."
    exit 1
fi
```

**Run verification script**:
```bash
chmod +x rustydb-verify-installation.sh
sudo ./rustydb-verify-installation.sh
```

---

## Upgrade Procedures

### Upgrading from v0.5.0 to v0.5.1

RustyDB v0.5.1 is backward compatible with v0.5.0 data files.

#### Pre-Upgrade Checklist

- [ ] Review [Release Notes](./RELEASE_NOTES.md) for breaking changes
- [ ] Perform full database backup
- [ ] Verify backup integrity
- [ ] Test upgrade in non-production environment
- [ ] Schedule maintenance window
- [ ] Notify users of planned downtime
- [ ] Document current configuration
- [ ] Verify disk space availability (2x current data size)

#### Upgrade Methods

##### Method 1: In-Place Upgrade (Single Instance)

**Step 1: Backup Current Installation**

```bash
# Stop database server
sudo systemctl stop rustydb

# Backup data directory
sudo tar -czf /backup/rustydb-data-$(date +%Y%m%d_%H%M%S).tar.gz \
  /var/lib/rustydb/

# Backup configuration
sudo cp -a /etc/rustydb /backup/rustydb-config-$(date +%Y%m%d_%H%M%S)

# Backup binaries
sudo cp -a /opt/rustydb/bin /backup/rustydb-bin-$(date +%Y%m%d_%H%M%S)
```

**Step 2: Install New Binaries**

```bash
# Download or build v0.5.1 binaries
cd /path/to/rusty-db
git fetch --tags
git checkout v0.5.1
cargo build --release

# Backup old binaries
sudo mv /opt/rustydb/bin/rusty-db-server /opt/rustydb/bin/rusty-db-server.v0.5.0
sudo mv /opt/rustydb/bin/rusty-db-cli /opt/rustydb/bin/rusty-db-cli.v0.5.0

# Install new binaries
sudo cp target/release/rusty-db-server /opt/rustydb/bin/
sudo cp target/release/rusty-db-cli /opt/rustydb/bin/
sudo chown rustydb:rustydb /opt/rustydb/bin/*
sudo chmod 750 /opt/rustydb/bin/*

# Verify version
/opt/rustydb/bin/rusty-db-server --version
# Expected: v0.5.1
```

**Step 3: Update Configuration (if needed)**

```bash
# Review configuration changes in release notes
# Update /etc/rustydb/rustydb.toml if necessary
sudo -u rustydb nano /etc/rustydb/rustydb.toml
```

**Step 4: Start Server and Verify**

```bash
# Start database server
sudo systemctl start rustydb

# Check status
sudo systemctl status rustydb

# Monitor logs for errors
sudo journalctl -u rustydb -f

# Verify version
curl http://localhost:8080/health | jq '.version'
# Expected: "0.5.1"

# Test functionality
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ serverInfo { version } }"}'
```

**Step 5: Verify Data Integrity**

```bash
# Run integrity checks (if available)
# Query existing data
# Run test transactions
# Verify application functionality
```

**Rollback Procedure (if upgrade fails)**:

```bash
# Stop new version
sudo systemctl stop rustydb

# Restore old binaries
sudo cp /opt/rustydb/bin/rusty-db-server.v0.5.0 /opt/rustydb/bin/rusty-db-server
sudo cp /opt/rustydb/bin/rusty-db-cli.v0.5.0 /opt/rustydb/bin/rusty-db-cli

# Restore configuration (if changed)
sudo rm -rf /etc/rustydb
sudo cp -a /backup/rustydb-config-TIMESTAMP /etc/rustydb

# Restore data (if corrupted)
sudo rm -rf /var/lib/rustydb
sudo tar -xzf /backup/rustydb-data-TIMESTAMP.tar.gz -C /

# Start old version
sudo systemctl start rustydb

# Verify rollback
/opt/rustydb/bin/rusty-db-server --version
# Expected: v0.5.0
```

##### Method 2: Blue-Green Deployment (Zero Downtime)

For production environments requiring zero downtime:

**Step 1: Prepare New Environment**

```bash
# Set up new server (green) with v0.5.1
# Install RustyDB v0.5.1 following installation procedures
# Configure replication from existing server (blue)
```

**Step 2: Replicate Data**

```bash
# Configure replication (details depend on replication setup)
# Ensure green server is synchronized with blue server
# Verify data consistency
```

**Step 3: Switch Traffic**

```bash
# Update load balancer to point to green server
# Or update DNS records
# Monitor for errors
```

**Step 4: Decommission Blue Server**

```bash
# After verification period, stop blue server
# Keep as backup for rollback period
```

##### Method 3: Rolling Upgrade (Clustered Environment)

For clustered deployments:

**Step 1: Upgrade Standby Nodes**

```bash
# For each standby node:
# 1. Stop node
# 2. Upgrade binaries
# 3. Start node
# 4. Verify synchronization
# 5. Wait for full replication catch-up
# 6. Proceed to next node
```

**Step 2: Failover to Upgraded Standby**

```bash
# Perform controlled failover to upgraded standby
# Old primary becomes new standby
```

**Step 3: Upgrade Former Primary**

```bash
# Upgrade former primary node
# Verify cluster consistency
```

### Upgrade Verification

```bash
# Verify all nodes running v0.5.1
/opt/rustydb/bin/rusty-db-server --version

# Check cluster status (if clustered)
curl http://localhost:8080/cluster/status

# Verify data integrity
# Run test queries
# Check application functionality

# Monitor performance metrics
curl http://localhost:9100/metrics
```

---

## Uninstallation

### Complete Uninstallation (Remove All Data)

**Warning**: This will permanently delete all database data.

#### Linux

```bash
# Step 1: Stop service
sudo systemctl stop rustydb
sudo systemctl disable rustydb

# Step 2: Remove service file
sudo rm /etc/systemd/system/rustydb.service
sudo systemctl daemon-reload

# Step 3: Remove binaries
sudo rm -rf /opt/rustydb

# Step 4: Remove data (DESTRUCTIVE)
sudo rm -rf /var/lib/rustydb

# Step 5: Remove logs
sudo rm -rf /var/log/rustydb

# Step 6: Remove configuration
sudo rm -rf /etc/rustydb

# Step 7: Remove runtime files
sudo rm -rf /run/rustydb

# Step 8: Remove cache
sudo rm -rf /var/cache/rustydb

# Step 9: Remove user and group
sudo userdel rustydb
sudo groupdel rustydb

# Step 10: Remove symlinks
sudo rm /usr/local/bin/rusty-db-server
sudo rm /usr/local/bin/rusty-db-cli

# Step 11: Clean up systemd remnants
sudo rm /etc/systemd/system/disable-thp.service
sudo systemctl daemon-reload

# Step 12: Remove logrotate configuration
sudo rm /etc/logrotate.d/rustydb

# Step 13: Remove sysctl configuration
sudo rm /etc/sysctl.d/99-rustydb.conf

# Step 14: Remove ulimits configuration
sudo rm /etc/security/limits.d/99-rustydb.conf
```

#### Windows

```powershell
# Stop and remove service
nssm stop RustyDB
nssm remove RustyDB confirm

# Remove binaries
Remove-Item -Recurse -Force "C:\Program Files\RustyDB"

# Remove data (DESTRUCTIVE)
Remove-Item -Recurse -Force "C:\ProgramData\RustyDB"

# Remove from PATH
$path = [Environment]::GetEnvironmentVariable("Path", "Machine")
$path = ($path.Split(';') | Where-Object { $_ -ne "C:\Program Files\RustyDB\bin" }) -join ';'
[Environment]::SetEnvironmentVariable("Path", $path, "Machine")

# Remove user account
Remove-LocalUser -Name "rustydb"
```

### Uninstallation with Data Preservation

If you want to preserve data for potential reinstallation:

```bash
# Step 1: Stop service
sudo systemctl stop rustydb
sudo systemctl disable rustydb

# Step 2: Backup data
sudo tar -czf /backup/rustydb-data-final-$(date +%Y%m%d_%H%M%S).tar.gz \
  /var/lib/rustydb/

# Step 3: Remove binaries and configuration
sudo rm -rf /opt/rustydb
sudo rm -rf /etc/rustydb

# Step 4: Remove service files
sudo rm /etc/systemd/system/rustydb.service
sudo systemctl daemon-reload

# Step 5: Move data to backup location (instead of deleting)
sudo mv /var/lib/rustydb /backup/rustydb-data-preserved

# Step 6: Keep logs for audit purposes
sudo mv /var/log/rustydb /backup/rustydb-logs-preserved

# Data can be restored later by:
# sudo mv /backup/rustydb-data-preserved /var/lib/rustydb
```

---

## Troubleshooting

### Installation Issues

#### Issue: Binary Won't Execute

**Symptoms**:
```
bash: /opt/rustydb/bin/rusty-db-server: cannot execute binary file: Exec format error
```

**Causes**:
- Wrong architecture (32-bit vs 64-bit, x86_64 vs ARM64)
- Corrupted binary download
- Incorrect platform binary

**Solutions**:
```bash
# Check binary type
file /opt/rustydb/bin/rusty-db-server

# Expected: ELF 64-bit LSB executable, x86-64

# Check system architecture
uname -m
# Expected: x86_64

# Re-download or rebuild binary for correct platform
```

#### Issue: Missing Dependencies

**Symptoms**:
```
error while loading shared libraries: libssl.so.1.1: cannot open shared object file
```

**Solutions**:
```bash
# Check missing dependencies
ldd /opt/rustydb/bin/rusty-db-server

# Install missing libraries (Debian/Ubuntu)
sudo apt-get install -y libssl1.1

# Or (RHEL/CentOS)
sudo yum install -y openssl-libs

# Or rebuild with static linking
cargo build --release --features static-linking
```

#### Issue: Permission Denied

**Symptoms**:
```
Permission denied when accessing /var/lib/rustydb/data
```

**Solutions**:
```bash
# Fix ownership
sudo chown -R rustydb:rustydb /var/lib/rustydb

# Fix permissions
sudo chmod 700 /var/lib/rustydb/data

# Verify
ls -la /var/lib/rustydb/
```

### Service Startup Issues

#### Issue: Service Fails to Start

**Check systemd logs**:
```bash
sudo journalctl -u rustydb -n 100 --no-pager

# Look for error messages
sudo systemctl status rustydb -l
```

**Common causes and solutions**:

1. **Port already in use**:
   ```bash
   sudo lsof -i :5432
   sudo lsof -i :8080

   # Kill conflicting process or change port
   ```

2. **Data directory not found**:
   ```bash
   sudo mkdir -p /var/lib/rustydb/data
   sudo chown -R rustydb:rustydb /var/lib/rustydb
   ```

3. **Configuration file errors**:
   ```bash
   # Validate TOML syntax
   # Fix syntax errors in /etc/rustydb/rustydb.toml
   ```

### Upgrade Issues

#### Issue: Data Format Incompatibility

**Symptoms**:
- Server fails to start after upgrade
- Error messages about unsupported data format

**Solutions**:
```bash
# Rollback to previous version
sudo systemctl stop rustydb
sudo cp /backup/rustydb-bin-TIMESTAMP/* /opt/rustydb/bin/
sudo systemctl start rustydb

# Contact support for migration assistance
```

#### Issue: Configuration Migration Errors

**Symptoms**:
- Server starts but behaves incorrectly
- Warning messages about unknown configuration options

**Solutions**:
```bash
# Review release notes for configuration changes
# Update configuration file
# Restart service
sudo systemctl restart rustydb
```

### Getting Help

#### Collect Diagnostic Information

```bash
# Create diagnostic bundle
mkdir -p /tmp/rustydb-diagnostics
cd /tmp/rustydb-diagnostics

# System information
uname -a > system-info.txt
lsb_release -a >> system-info.txt 2>/dev/null

# RustyDB version
/opt/rustydb/bin/rusty-db-server --version > version.txt

# Service status
systemctl status rustydb -l > service-status.txt

# Recent logs
journalctl -u rustydb -n 500 --no-pager > systemd-logs.txt
tail -n 500 /var/log/rustydb/rustydb.log > application-logs.txt

# Configuration
cp /etc/rustydb/rustydb.toml config.toml

# Process information
ps aux | grep rusty-db-server > process-info.txt

# Network information
netstat -tlnp | grep rusty-db-server > network-info.txt

# System resources
free -h > memory-info.txt
df -h > disk-info.txt

# Create archive
cd ..
tar -czf rustydb-diagnostics-$(date +%Y%m%d_%H%M%S).tar.gz rustydb-diagnostics/
echo "Diagnostic bundle created: rustydb-diagnostics-$(date +%Y%m%d_%H%M%S).tar.gz"
```

#### Support Channels

- **GitHub Issues**: https://github.com/harborgrid-justin/rusty-db/issues
- **Documentation**: `/home/user/rusty-db/docs/`
- **Release Notes**: [RELEASE_NOTES.md](./RELEASE_NOTES.md)
- **Security Issues**: Report via GitHub Security Advisories

---

## Appendices

### Appendix A: Directory Reference

| Directory | Purpose | Permissions | Owner |
|-----------|---------|-------------|-------|
| `/opt/rustydb/` | Installation root | 750 | rustydb:rustydb |
| `/opt/rustydb/bin/` | Binaries | 750 | rustydb:rustydb |
| `/var/lib/rustydb/` | Data root | 700 | rustydb:rustydb |
| `/var/lib/rustydb/data/` | Database data | 700 | rustydb:rustydb |
| `/var/lib/rustydb/wal/` | WAL segments | 700 | rustydb:rustydb |
| `/var/lib/rustydb/backup/` | Backups | 700 | rustydb:rustydb |
| `/var/log/rustydb/` | Log files | 755 | rustydb:rustydb |
| `/etc/rustydb/` | Configuration | 750 | rustydb:rustydb |
| `/etc/rustydb/secrets/` | Secrets (keys, certs) | 700 | rustydb:rustydb |
| `/run/rustydb/` | Runtime files (PID) | 755 | rustydb:rustydb |
| `/var/cache/rustydb/` | Cache files | 755 | rustydb:rustydb |
| `/var/tmp/rustydb/` | Temporary files | 1777 | rustydb:rustydb |

### Appendix B: Port Reference

| Port | Protocol | Service | Default | Configurable |
|------|----------|---------|---------|--------------|
| 5432 | TCP | Database Server | Yes | Yes |
| 8080 | TCP | GraphQL/REST API | Yes | Yes |
| 9100 | TCP | Prometheus Metrics | Yes | Yes |
| 54321 | TCP | Alternative DB Port | No | Yes |

### Appendix C: Environment Variables

| Variable | Purpose | Default | Example |
|----------|---------|---------|---------|
| `RUSTYDB_DATA_DIR` | Data directory | `./data` | `/var/lib/rustydb/data` |
| `RUSTYDB_PORT` | Database port | `5432` | `5432` |
| `RUSTYDB_GRAPHQL_PORT` | GraphQL API port | `8080` | `8080` |
| `RUSTYDB_BUFFER_POOL_SIZE` | Buffer pool pages | `1000` | `131072` |
| `RUST_LOG` | Logging level | `info` | `debug`, `info`, `warn`, `error` |

### Appendix D: Configuration File Template Locations

| Platform | Location |
|----------|----------|
| Linux (FHS) | `/etc/rustydb/rustydb.toml` |
| Windows | `C:\ProgramData\RustyDB\conf\rustydb.toml` |
| Docker | `/etc/rustydb/rustydb.toml` |
| Source Repository | `conf/rustydb.toml` |

### Appendix E: Useful Commands Quick Reference

```bash
# Installation
cargo build --release
sudo cp target/release/rusty-db-server /opt/rustydb/bin/

# Service Management
sudo systemctl start rustydb
sudo systemctl stop rustydb
sudo systemctl restart rustydb
sudo systemctl status rustydb
sudo systemctl enable rustydb

# Monitoring
sudo journalctl -u rustydb -f
tail -f /var/log/rustydb/rustydb.log
curl http://localhost:8080/health
curl http://localhost:9100/metrics

# Diagnostics
/opt/rustydb/bin/rusty-db-server --version
ps aux | grep rusty-db-server
netstat -tlnp | grep rusty-db-server
lsof -i :5432

# Backup
sudo systemctl stop rustydb
sudo tar -czf backup.tar.gz /var/lib/rustydb/
sudo systemctl start rustydb

# Upgrade
sudo systemctl stop rustydb
sudo cp new-binary /opt/rustydb/bin/rusty-db-server
sudo systemctl start rustydb
```

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-12-27 | Enterprise Documentation Agent 1 | Initial release for RustyDB v0.5.1 |

---

**Copyright © 2025 RustyDB Project. All rights reserved.**

**RustyDB v0.5.1 - Enterprise Edition**

Built with Rust for Safety, Performance, and Reliability
