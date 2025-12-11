# RustyDB Deployment Guide

**Document Version**: 1.0
**Last Updated**: 2025-12-11
**Classification**: Internal
**Maintained By**: Infrastructure Team

---

## Executive Summary

This guide provides comprehensive instructions for deploying RustyDB in various environments, from single-node installations to highly available multi-datacenter clusters. It covers system requirements, installation procedures, configuration, networking, scaling strategies, and upgrade procedures.

### ⚠️ Implementation Status Notice

**Last Verified**: 2025-12-11

This deployment guide describes the **target architecture and planned features** for RustyDB. While the core database engine is operational, many advanced deployment features are still in development.

**Current Implementation Status:**

| Feature Category | Status | Notes |
|-----------------|--------|-------|
| **Core Database Engine** | ✅ Working | Transaction management, MVCC, basic operations |
| **GraphQL API** | ✅ Working | 69.3% test pass rate (70/101 tests) |
| **Transaction System** | ✅ Working | 4 isolation levels fully tested |
| **MVCC Snapshots** | ✅ Working | 100% test pass rate (25/25 tests) |
| **Security Modules** | ✅ Implemented | 17 modules verified in codebase |
| **Simple Configuration** | ✅ Working | 4 basic config options (port, data_dir, page_size, buffer_pool) |
| **File-based Config** | ⚠️ Planned | Extensive .conf file parsing not yet implemented |
| **Binary Installation** | ⚠️ Planned | Package repositories not yet available |
| **Clustering** | ⚠️ In Development | Modules exist but integration incomplete |
| **Replication** | ⚠️ In Development | Modules exist but not fully tested |
| **Container Images** | ⚠️ Planned | Docker images not yet published |
| **K8s Operators** | ⚠️ Planned | Kubernetes operators not yet available |

**Recommended Current Use Cases:**
- ✅ Development and testing
- ✅ GraphQL API integration testing
- ✅ Transaction and MVCC behavior validation
- ✅ Security module evaluation
- ⚠️ Production deployment (single-node only, extensive testing required)

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Pre-Installation Planning](#pre-installation-planning)
3. [Installation Methods](#installation-methods)
4. [Configuration](#configuration)
5. [Network Setup](#network-setup)
6. [Security Hardening](#security-hardening)
7. [Single-Node Deployment](#single-node-deployment)
8. [High Availability Cluster](#high-availability-cluster)
9. [Multi-Datacenter Deployment](#multi-datacenter-deployment)
10. [Container Deployment](#container-deployment)
11. [Kubernetes Deployment](#kubernetes-deployment)
12. [Cloud Deployments](#cloud-deployments)
13. [Scaling Strategies](#scaling-strategies)
14. [Upgrade Procedures](#upgrade-procedures)
15. [Rollback Procedures](#rollback-procedures)
16. [Post-Deployment Validation](#post-deployment-validation)

---

## System Requirements

### Minimum Requirements

**Development/Testing Environment:**

- **CPU**: 2 cores (x86-64)
- **RAM**: 4 GB
- **Storage**: 20 GB SSD
- **Network**: 100 Mbps
- **OS**: Linux (Ubuntu 20.04+, RHEL 8+, Debian 11+)

**Small Production Environment:**

- **CPU**: 4 cores (x86-64 with AVX2)
- **RAM**: 8 GB
- **Storage**: 100 GB SSD (NVMe preferred)
- **Network**: 1 Gbps
- **OS**: Linux (Ubuntu 22.04 LTS, RHEL 9)

### Recommended Requirements

**Medium Production Environment:**

- **CPU**: 8-16 cores (x86-64 with AVX2/AVX-512)
- **RAM**: 32-64 GB
- **Storage**: 500 GB - 2 TB NVMe SSD (RAID 10)
- **Network**: 10 Gbps
- **OS**: Linux (Ubuntu 22.04 LTS, RHEL 9)

**Large Production Environment:**

- **CPU**: 32-64 cores (x86-64 with AVX-512)
- **RAM**: 128-512 GB
- **Storage**: 2-10 TB NVMe SSD (RAID 10 or distributed storage)
- **Network**: 25-100 Gbps
- **OS**: Linux (Ubuntu 22.04 LTS, RHEL 9) with RT kernel

### High Availability Cluster Requirements

**Per Node:**

- Meets recommended requirements for expected workload
- Dedicated network interface for cluster heartbeat (1-10 Gbps)
- Shared storage or replicated storage
- Minimum 3 nodes for quorum

### Storage Requirements

**Data Storage:**
- SSD required (NVMe preferred for production)
- Sustained IOPS: 10,000+ for SSD, 100,000+ for NVMe
- Latency: < 10ms for SSD, < 1ms for NVMe

**Backup Storage:**
- Can use slower storage (HDD acceptable)
- Minimum 2x data size for full backups
- Offsite/cloud storage recommended

**Log Storage:**
- Fast SSD/NVMe for WAL logs
- Separate volume from data for optimal performance
- Size: 10-20% of data size

### Operating System Support

| OS | Version | Status | Notes |
|----|---------|--------|-------|
| Ubuntu | 20.04 LTS, 22.04 LTS, 24.04 LTS | ✅ Supported | Recommended |
| RHEL | 8.x, 9.x | ✅ Supported | Enterprise standard |
| CentOS Stream | 9 | ✅ Supported | |
| Debian | 11, 12 | ✅ Supported | |
| Amazon Linux | 2, 2023 | ✅ Supported | AWS optimized |
| Rocky Linux | 8.x, 9.x | ✅ Supported | RHEL alternative |
| Windows Server | 2019, 2022 | ⚠️ Experimental | Limited support |

### Software Dependencies

**Required:**
- Rust 1.70+ (for compilation)
- GCC/Clang (for native dependencies)
- OpenSSL 1.1.1+ or 3.0+
- systemd (for service management)

**Optional:**
- Docker 24.0+ (for containerized deployment)
- Kubernetes 1.27+ (for orchestrated deployment)
- Ansible 2.15+ (for automation)

---

## Pre-Installation Planning

### Capacity Planning

1. **Estimate Data Size**
   - Current data volume
   - Growth rate (daily/monthly)
   - Retention policy
   - Index size (typically 20-30% of data size)

2. **Calculate Storage Requirements**
   ```
   Total Storage = (Data Size + Index Size) × Compression Factor × Replication Factor + WAL Space

   Example:
   - Data: 1 TB
   - Indexes: 300 GB
   - Compression: 0.5 (50% compression)
   - Replication: 3 copies
   - WAL: 100 GB

   Total = (1000 + 300) × 0.5 × 3 + 100 = 2,050 GB ≈ 2.1 TB
   ```

3. **Calculate Memory Requirements**
   ```
   Buffer Pool = Working Set × 1.2
   System Memory = Buffer Pool + OS (4-8 GB) + Application Overhead (20%)

   Example:
   - Working Set: 50 GB
   - Buffer Pool: 60 GB
   - OS: 8 GB
   - Overhead: 13.6 GB

   Total RAM = 60 + 8 + 13.6 = 81.6 GB ≈ 96 GB (with headroom)
   ```

4. **Network Bandwidth**
   ```
   Bandwidth = Peak TPS × Average Transaction Size × Replication Factor

   Example:
   - Peak TPS: 10,000
   - Avg Transaction: 2 KB
   - Replication: 3

   Bandwidth = 10,000 × 2 KB × 3 = 60 MB/s ≈ 480 Mbps
   ```

### Architecture Selection

#### Single-Node Architecture

**Use Cases:**
- Development/testing
- Small applications (< 100 concurrent users)
- Non-critical workloads

**Pros:**
- Simple setup
- Low cost
- Easy maintenance

**Cons:**
- No high availability
- Limited scalability
- Single point of failure

#### Primary-Standby Architecture

**Use Cases:**
- Production applications
- 99.9% availability requirement
- Read-heavy workloads

**Pros:**
- High availability
- Read scaling with standby
- Automatic failover

**Cons:**
- Write bottleneck on primary
- Replication lag possible
- More complex setup

#### Multi-Master Cluster

**Use Cases:**
- Mission-critical applications
- 99.99%+ availability requirement
- Global distribution
- High write throughput

**Pros:**
- No single point of failure
- Write scalability
- Geographic distribution
- Automatic failover

**Cons:**
- Complex setup
- Conflict resolution needed
- Higher latency for distributed writes

### Network Planning

1. **Network Topology**
   - Application network (client access)
   - Cluster network (inter-node communication)
   - Storage network (SAN/NAS access)
   - Management network (administration)

2. **Port Requirements**
   ```
   5432  - Database client connections (TCP)
   5433  - Replication connections (TCP)
   7000  - Cluster coordination (TCP)
   7001  - Cluster gossip (UDP)
   8080  - REST API (TCP)
   9090  - Metrics (Prometheus)
   9187  - Database exporter
   ```

3. **Firewall Rules**
   - Allow client access from application network
   - Allow cluster traffic between database nodes
   - Restrict administrative access to management network
   - Allow monitoring from monitoring network

---

## Installation Methods

### Method 1: Binary Installation (Recommended)

#### Ubuntu/Debian

```bash
# Add RustyDB repository
curl -fsSL https://packages.rustydb.io/gpg | sudo gpg --dearmor -o /usr/share/keyrings/rustydb-archive-keyring.gpg

echo "deb [signed-by=/usr/share/keyrings/rustydb-archive-keyring.gpg] https://packages.rustydb.io/apt stable main" | \
  sudo tee /etc/apt/sources.list.d/rustydb.list

# Update package index
sudo apt-get update

# Install RustyDB
sudo apt-get install rusty-db

# Install additional tools
sudo apt-get install rusty-db-client rusty-db-tools
```

#### RHEL/CentOS/Rocky Linux

```bash
# Add RustyDB repository
sudo cat > /etc/yum.repos.d/rustydb.repo << 'EOF'
[rustydb]
name=RustyDB Repository
baseurl=https://packages.rustydb.io/rpm/stable/$basearch
enabled=1
gpgcheck=1
gpgkey=https://packages.rustydb.io/gpg
EOF

# Install RustyDB
sudo yum install rusty-db

# Install additional tools
sudo yum install rusty-db-client rusty-db-tools
```

### Method 2: Source Installation

```bash
# Install build dependencies
# Ubuntu/Debian:
sudo apt-get install build-essential curl libssl-dev pkg-config

# RHEL/CentOS:
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone repository
git clone https://github.com/rustydb/rusty-db.git
cd rusty-db

# Build release version
cargo build --release

# Install binaries
sudo cp target/release/rusty-db-server /usr/local/bin/
sudo cp target/release/rusty-db-cli /usr/local/bin/
sudo cp target/release/rusty-db-backup /usr/local/bin/

# Create system user
sudo useradd -r -s /bin/false rustydb

# Create directories
sudo mkdir -p /var/lib/rusty-db
sudo mkdir -p /var/log/rusty-db
sudo mkdir -p /etc/rusty-db

# Set permissions
sudo chown -R rustydb:rustydb /var/lib/rusty-db
sudo chown -R rustydb:rustydb /var/log/rusty-db
```

### Method 3: Docker Installation

```bash
# Pull official image
docker pull rustydb/rusty-db:latest

# Run container
docker run -d \
  --name rusty-db \
  -p 5432:5432 \
  -v /var/lib/rusty-db:/var/lib/rusty-db \
  -e RUSTYDB_PASSWORD=secure_password \
  rustydb/rusty-db:latest
```

---

## Configuration

### Initial Configuration

**⚠️ Configuration Status**: The extensive configuration options shown below represent the **planned/target configuration system**. The current implementation uses a simplified `Config` struct with only 4 fields (see `src/lib.rs`):

```rust
pub struct Config {
    pub data_dir: String,      // Default: "./data"
    pub page_size: usize,      // Default: 4096 bytes
    pub buffer_pool_size: usize, // Default: 1000 pages
    pub port: u16,             // Default: 5432
}
```

**Current Status**: The server can be started with default configuration or programmatically configured in code. File-based configuration and the extensive options below are **planned features** not yet fully implemented.

### Target Configuration (Planned)

The following configuration represents the planned enterprise configuration system:

```bash
sudo cat > /etc/rusty-db/rusty-db.conf << 'EOF'
# RustyDB Configuration File
# Version: 1.0 (PLANNED - Not all options implemented)

# ============================================================================
# Connection Settings
# ============================================================================

# Port for client connections (✅ Implemented)
port = 5432

# Listen addresses (⚠️ Planned)
listen_addresses = "0.0.0.0"

# Maximum number of concurrent client connections (⚠️ Planned)
max_connections = 500

# Connection timeout (seconds) (⚠️ Planned)
connection_timeout = 30

# Idle connection timeout (seconds) (⚠️ Planned)
idle_connection_timeout = 300

# ============================================================================
# Memory Settings
# ============================================================================

# Buffer pool size in pages (✅ Implemented as buffer_pool_size: 1000 pages)
# Note: Current implementation uses page count, not MB
buffer_pool_size_mb = 8192  # ⚠️ Planned: Convert to MB-based sizing

# Enable slab allocator for better memory management
slab_allocator_enabled = true

# Enable arena allocator for per-query contexts
arena_allocator_enabled = true

# Enable huge pages (requires OS configuration)
huge_pages_enabled = true

# Memory pressure threshold (percentage)
memory_pressure_threshold = 85

# ============================================================================
# Storage Settings
# ============================================================================

# Data directory (✅ Implemented as data_dir)
data_directory = "/var/lib/rusty-db/data"

# WAL (Write-Ahead Log) directory (⚠️ Planned)
wal_directory = "/var/lib/rusty-db/wal"

# Page size (bytes) - DO NOT CHANGE after initialization (✅ Implemented)
page_size = 4096

# Enable direct I/O (bypasses OS page cache)
direct_io = true

# Enable fsync for durability (disable only for non-critical data)
fsync = true

# Checkpoint interval (seconds)
checkpoint_interval = 300

# ============================================================================
# Performance Settings
# ============================================================================

# Number of parallel workers for queries
parallel_workers = 4

# Prefetch size (pages)
prefetch_size = 128

# Enable JIT compilation for queries
jit_enabled = true

# Enable vectorized execution
vectorized_execution = true

# Enable SIMD optimizations (requires CPU support)
simd_enabled = true

# ============================================================================
# Replication Settings
# ============================================================================
# ⚠️ Status: Replication modules exist in codebase but configuration
#           integration not fully tested

# Replication mode: off, async, sync, semi-sync (⚠️ Planned)
replication_mode = "async"

# Archive mode for point-in-time recovery
archive_mode = false

# Archive destination directory
archive_dest = "/var/lib/rusty-db/archive"

# Maximum replication lag (seconds) for sync/semi-sync
max_replication_lag = 10

# Replication bandwidth limit (MB/s, 0 = unlimited)
replication_bandwidth_mbps = 1000

# ============================================================================
# Security Settings
# ============================================================================
# ⚠️ Status: Security modules implemented (17 modules verified)
#           Configuration integration in progress

# Enable SSL/TLS (⚠️ Planned)
ssl_enabled = true

# SSL certificate file
ssl_cert_file = "/etc/rusty-db/certs/server.crt"

# SSL private key file
ssl_key_file = "/etc/rusty-db/certs/server.key"

# SSL CA certificate file
ssl_ca_file = "/etc/rusty-db/certs/ca.crt"

# Require client certificates
ssl_require_client_cert = false

# Enable encryption at rest
encryption_at_rest = true

# Master encryption key location (HSM/KMS path or file)
master_encryption_key = "/etc/rusty-db/keys/master.key"

# Enable audit logging
audit_enabled = true

# Audit log destination
audit_log_dest = "/var/log/rusty-db/audit.log"

# Password hashing algorithm: argon2id, bcrypt, scrypt
password_hash_algorithm = "argon2id"

# Minimum password length
min_password_length = 12

# Password complexity requirements
password_require_uppercase = true
password_require_lowercase = true
password_require_numbers = true
password_require_special = true

# Failed login threshold before account lock
failed_login_threshold = 5

# Account lockout duration (seconds)
account_lockout_duration = 900

# ============================================================================
# Logging Settings
# ============================================================================

# Log level: debug, info, warn, error
log_level = "info"

# Log destination: file, syslog, console
log_destination = "file"

# Log directory
log_directory = "/var/log/rusty-db"

# Log file rotation size (MB)
log_rotation_size_mb = 100

# Log file retention (days)
log_retention_days = 30

# Enable slow query logging
slow_query_log_enabled = true

# Slow query threshold (milliseconds)
slow_query_threshold_ms = 1000

# Log all DDL statements
log_ddl_statements = true

# ============================================================================
# Monitoring Settings
# ============================================================================

# Enable metrics collection
metrics_enabled = true

# Metrics port (Prometheus format)
metrics_port = 9090

# Metrics collection interval (seconds)
metrics_interval = 10

# Enable performance monitoring
performance_monitoring = true

# Enable query execution statistics
query_statistics = true

# ============================================================================
# Clustering Settings
# ============================================================================

# Enable clustering
cluster_enabled = false

# Cluster name
cluster_name = "rustydb-cluster"

# Node name (unique within cluster)
node_name = "node1"

# Cluster members (comma-separated: host:port)
cluster_members = "node1:7000,node2:7000,node3:7000"

# Cluster coordination port
cluster_port = 7000

# Cluster gossip port
cluster_gossip_port = 7001

# Heartbeat interval (milliseconds)
cluster_heartbeat_interval = 1000

# Node timeout (milliseconds)
cluster_node_timeout = 5000

# ============================================================================
# Backup Settings
# ============================================================================

# Backup directory
backup_directory = "/var/lib/rusty-db/backups"

# Enable backup compression
backup_compression = true

# Backup compression algorithm: lz4, zstd, snappy
backup_compression_algorithm = "zstd"

# Backup compression level (1-22 for zstd)
backup_compression_level = 3

# Enable backup encryption
backup_encryption = true

# Backup retention (days)
backup_retention_days = 30

# ============================================================================
# Advanced Settings
# ============================================================================

# Enable lock-free data structures
lock_free_structures = true

# Work stealing scheduler
work_stealing_enabled = true

# Enable adaptive query execution
adaptive_query_execution = true

# Enable query result caching
query_cache_enabled = true

# Query cache size (MB)
query_cache_size_mb = 1024

# Enable automatic vacuuming
auto_vacuum = true

# Vacuum threshold (percentage of dead tuples)
vacuum_threshold_percent = 20

# Enable automatic statistics gathering
auto_statistics_gather = true

# Statistics gather interval
statistics_gather_interval = "1 day"
EOF
```

### Security Configuration

#### Generate SSL Certificates

```bash
# Create certificate directory
sudo mkdir -p /etc/rusty-db/certs
cd /etc/rusty-db/certs

# Generate CA key and certificate
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 3650 -key ca.key -out ca.crt \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=RustyDB CA"

# Generate server key and certificate
openssl genrsa -out server.key 4096
openssl req -new -key server.key -out server.csr \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=rustydb.company.com"

# Sign server certificate with CA
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out server.crt -days 365 \
  -extfile <(printf "subjectAltName=DNS:rustydb.company.com,DNS:localhost,IP:127.0.0.1")

# Set permissions
sudo chown rustydb:rustydb /etc/rusty-db/certs/*
sudo chmod 600 /etc/rusty-db/certs/*.key
sudo chmod 644 /etc/rusty-db/certs/*.crt
```

#### Generate Master Encryption Key

```bash
# Create keys directory
sudo mkdir -p /etc/rusty-db/keys

# Generate master encryption key (256-bit)
openssl rand -hex 32 | sudo tee /etc/rusty-db/keys/master.key

# Set permissions
sudo chown rustydb:rustydb /etc/rusty-db/keys/master.key
sudo chmod 400 /etc/rusty-db/keys/master.key
```

### Database Initialization

```bash
# Initialize database cluster
sudo -u rustydb rusty-db-server --init \
  --data-dir /var/lib/rusty-db/data \
  --config /etc/rusty-db/rusty-db.conf

# Create admin user
sudo -u rustydb rusty-db-cli --command "
CREATE USER admin WITH PASSWORD 'secure_admin_password' SUPERUSER;
"

# Create application database
sudo -u rustydb rusty-db-cli --command "
CREATE DATABASE myapp;
"

# Create application user with limited privileges
sudo -u rustydb rusty-db-cli --command "
CREATE USER appuser WITH PASSWORD 'secure_app_password';
GRANT CONNECT ON DATABASE myapp TO appuser;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO appuser;
"
```

---

## Network Setup

### Firewall Configuration

#### Ubuntu/Debian (UFW)

```bash
# Enable firewall
sudo ufw enable

# Allow SSH (if needed)
sudo ufw allow 22/tcp

# Allow database connections from application servers
sudo ufw allow from 10.0.1.0/24 to any port 5432 proto tcp

# Allow cluster traffic between database nodes
sudo ufw allow from 10.0.2.0/24 to any port 7000:7001 proto tcp
sudo ufw allow from 10.0.2.0/24 to any port 7001 proto udp

# Allow monitoring from monitoring servers
sudo ufw allow from 10.0.3.0/24 to any port 9090 proto tcp

# Reload firewall
sudo ufw reload

# Verify rules
sudo ufw status numbered
```

#### RHEL/CentOS (firewalld)

```bash
# Start firewalld
sudo systemctl start firewalld
sudo systemctl enable firewalld

# Create RustyDB service definition
sudo cat > /etc/firewalld/services/rustydb.xml << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<service>
  <short>RustyDB</short>
  <description>RustyDB Database Server</description>
  <port protocol="tcp" port="5432"/>
</service>
EOF

# Create cluster service definition
sudo cat > /etc/firewalld/services/rustydb-cluster.xml << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<service>
  <short>RustyDB Cluster</short>
  <description>RustyDB Cluster Communication</description>
  <port protocol="tcp" port="7000"/>
  <port protocol="tcp" port="7001"/>
  <port protocol="udp" port="7001"/>
</service>
EOF

# Reload firewalld
sudo firewall-cmd --reload

# Add services to public zone
sudo firewall-cmd --permanent --zone=public --add-service=rustydb
sudo firewall-cmd --permanent --zone=public --add-service=rustydb-cluster

# Allow from specific sources
sudo firewall-cmd --permanent --zone=public --add-rich-rule='
  rule family="ipv4"
  source address="10.0.1.0/24"
  service name="rustydb"
  accept'

# Reload firewalld
sudo firewall-cmd --reload
```

### Load Balancer Configuration

#### HAProxy Example

```bash
# Install HAProxy
sudo apt-get install haproxy

# Configure HAProxy
sudo cat > /etc/haproxy/haproxy.cfg << 'EOF'
global
    log /dev/log local0
    log /dev/log local1 notice
    chroot /var/lib/haproxy
    stats socket /run/haproxy/admin.sock mode 660 level admin
    stats timeout 30s
    user haproxy
    group haproxy
    daemon

defaults
    log global
    mode tcp
    option tcplog
    option dontlognull
    timeout connect 10s
    timeout client 1h
    timeout server 1h

# PostgreSQL wire protocol
listen rustydb_read_write
    bind *:5432
    mode tcp
    option pgsql-check user haproxy
    balance leastconn

    # Primary node (read-write)
    server primary1 10.0.2.101:5432 check port 5432 inter 2000 rise 2 fall 3

    # Standby nodes (backup, used if primary fails)
    server standby1 10.0.2.102:5432 check port 5432 inter 2000 rise 2 fall 3 backup
    server standby2 10.0.2.103:5432 check port 5432 inter 2000 rise 2 fall 3 backup

# Read-only queries (load balanced across all nodes)
listen rustydb_read_only
    bind *:5433
    mode tcp
    option pgsql-check user haproxy
    balance roundrobin

    server primary1 10.0.2.101:5432 check port 5432 inter 2000 rise 2 fall 3
    server standby1 10.0.2.102:5432 check port 5432 inter 2000 rise 2 fall 3
    server standby2 10.0.2.103:5432 check port 5432 inter 2000 rise 2 fall 3

# Statistics
listen stats
    bind *:8404
    mode http
    stats enable
    stats uri /stats
    stats refresh 30s
    stats auth admin:password
EOF

# Restart HAProxy
sudo systemctl restart haproxy
sudo systemctl enable haproxy
```

---

## Security Hardening

### Operating System Hardening

```bash
# Disable unnecessary services
sudo systemctl disable bluetooth
sudo systemctl disable cups

# Enable automatic security updates (Ubuntu/Debian)
sudo apt-get install unattended-upgrades
sudo dpkg-reconfigure -plow unattended-upgrades

# Configure kernel parameters
sudo cat >> /etc/sysctl.conf << 'EOF'
# Network security
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.tcp_syncookies = 1
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv4.conf.all.secure_redirects = 0
net.ipv4.conf.default.secure_redirects = 0

# Increase connection tracking
net.netfilter.nf_conntrack_max = 262144

# Performance tuning for database
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5
vm.overcommit_memory = 2
vm.overcommit_ratio = 90

# Shared memory for database
kernel.shmmax = 17179869184
kernel.shmall = 4194304
EOF

# Apply sysctl settings
sudo sysctl -p

# Configure transparent huge pages
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/defrag

# Make persistent
sudo cat > /etc/systemd/system/disable-thp.service << 'EOF'
[Unit]
Description=Disable Transparent Huge Pages

[Service]
Type=oneshot
ExecStart=/bin/sh -c "echo never > /sys/kernel/mm/transparent_hugepage/enabled"
ExecStart=/bin/sh -c "echo never > /sys/kernel/mm/transparent_hugepage/defrag"

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable disable-thp
sudo systemctl start disable-thp
```

### Database Hardening

```bash
# Restrict file permissions
sudo chmod 700 /var/lib/rusty-db
sudo chmod 750 /etc/rusty-db
sudo chmod 640 /etc/rusty-db/rusty-db.conf

# Configure pg_hba.conf (host-based authentication)
sudo cat > /etc/rusty-db/pg_hba.conf << 'EOF'
# TYPE  DATABASE        USER            ADDRESS                 METHOD

# Local connections
local   all             all                                     peer

# IPv4 local connections
host    all             all             127.0.0.1/32            scram-sha-256

# Application servers
host    all             appuser         10.0.1.0/24             scram-sha-256

# Admin access (VPN only)
host    all             admin           10.0.3.0/24             scram-sha-256 clientcert=verify-full

# Replication connections
host    replication     replicator      10.0.2.0/24             scram-sha-256

# Deny all others
host    all             all             0.0.0.0/0               reject
EOF

# Enable and configure AppArmor/SELinux
# Ubuntu (AppArmor)
sudo aa-enforce /etc/apparmor.d/usr.bin.rusty-db-server

# RHEL (SELinux)
sudo semanage port -a -t postgresql_port_t -p tcp 5432
sudo setsebool -P postgresql_can_rsync on
```

---

## Single-Node Deployment

### Quick Start

```bash
# 1. Install RustyDB (using binary installation)
sudo apt-get install rusty-db

# 2. Initialize database
sudo -u rustydb rusty-db-server --init

# 3. Create systemd service
sudo cat > /etc/systemd/system/rusty-db.service << 'EOF'
[Unit]
Description=RustyDB Database Server
Documentation=https://docs.rustydb.io
After=network.target

[Service]
Type=notify
User=rustydb
Group=rustydb

Environment="PGDATA=/var/lib/rusty-db/data"
Environment="RUSTYDB_CONFIG=/etc/rusty-db/rusty-db.conf"

ExecStart=/usr/bin/rusty-db-server --config /etc/rusty-db/rusty-db.conf
ExecReload=/bin/kill -HUP $MAINPID

KillMode=mixed
KillSignal=SIGINT

Restart=on-failure
RestartSec=10s

TimeoutStartSec=infinity
TimeoutStopSec=infinity

StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# 4. Start service
sudo systemctl daemon-reload
sudo systemctl enable rusty-db
sudo systemctl start rusty-db

# 5. Verify installation
rusty-db-cli --command "SELECT version();"
```

---

## High Availability Cluster

### Primary-Standby Setup

#### Primary Node Setup

```bash
# 1. Edit configuration on primary
sudo nano /etc/rusty-db/rusty-db.conf

# Add/modify:
replication_mode = "async"
archive_mode = true
archive_dest = "/var/lib/rusty-db/archive"

# 2. Create replication user
rusty-db-cli --command "
CREATE USER replicator WITH REPLICATION ENCRYPTED PASSWORD 'repl_password';
"

# 3. Configure pg_hba.conf for replication
echo "host replication replicator 10.0.2.0/24 scram-sha-256" | \
  sudo tee -a /etc/rusty-db/pg_hba.conf

# 4. Restart primary
sudo systemctl restart rusty-db

# 5. Create base backup for standby
rusty-db-backup --type pitr-base \
  --output /backups/standby-init.backup \
  --compress
```

#### Standby Node Setup

```bash
# 1. Copy base backup to standby
scp /backups/standby-init.backup standby1:/tmp/

# 2. On standby, restore base backup
sudo -u rustydb rusty-db-restore \
  --input /tmp/standby-init.backup \
  --data-dir /var/lib/rusty-db/data

# 3. Configure standby
sudo cat > /var/lib/rusty-db/data/recovery.conf << 'EOF'
standby_mode = on
primary_conninfo = 'host=primary1 port=5432 user=replicator password=repl_password'
restore_command = 'cp /var/lib/rusty-db/archive/%f %p'
trigger_file = '/var/lib/rusty-db/data/failover.trigger'
EOF

# 4. Start standby
sudo systemctl start rusty-db

# 5. Verify replication status on primary
rusty-db-cli --command "SELECT * FROM v$replication_status;"
```

### Multi-Node Cluster (Raft Consensus)

```bash
# Node 1 (Initial Leader)
sudo cat >> /etc/rusty-db/rusty-db.conf << 'EOF'
cluster_enabled = true
cluster_name = "production-cluster"
node_name = "node1"
cluster_members = "node1:7000,node2:7000,node3:7000"
cluster_port = 7000
cluster_gossip_port = 7001
EOF

sudo systemctl restart rusty-db

# Node 2
sudo cat >> /etc/rusty-db/rusty-db.conf << 'EOF'
cluster_enabled = true
cluster_name = "production-cluster"
node_name = "node2"
cluster_members = "node1:7000,node2:7000,node3:7000"
cluster_port = 7000
cluster_gossip_port = 7001
EOF

sudo systemctl restart rusty-db

# Node 3
sudo cat >> /etc/rusty-db/rusty-db.conf << 'EOF'
cluster_enabled = true
cluster_name = "production-cluster"
node_name = "node3"
cluster_members = "node1:7000,node2:7000,node3:7000"
cluster_port = 7000
cluster_gossip_port = 7001
EOF

sudo systemctl restart rusty-db

# Verify cluster status
rusty-db-cli --command "SELECT * FROM v$cluster_status;"
```

---

## Multi-Datacenter Deployment

### Active-Passive Configuration

```bash
# Primary Datacenter (DC1)
# Configure async replication to DR site

# Edit /etc/rusty-db/rusty-db.conf on primary:
replication_mode = "async"
archive_mode = true
archive_dest = "/var/lib/rusty-db/archive"

# DR Datacenter (DC2)
# Configure as standby

# On DR standby:
sudo cat > /var/lib/rusty-db/data/recovery.conf << 'EOF'
standby_mode = on
primary_conninfo = 'host=primary-dc1.company.com port=5432 user=replicator password=repl_password'
restore_command = 'cp /var/lib/rusty-db/archive/%f %p'
EOF

# Verify DR replication lag
rusty-db-cli --host primary-dc1.company.com --command "
SELECT
    standby_name,
    lag_seconds,
    estimated_data_loss_mb
FROM v$replication_status
WHERE standby_name = 'dr-dc2';
"
```

### Active-Active Configuration

```bash
# DC1 Configuration
cat >> /etc/rusty-db/rusty-db.conf << 'EOF'
cluster_enabled = true
cluster_name = "global-cluster"
node_name = "dc1-node1"
cluster_members = "dc1-node1:7000,dc1-node2:7000,dc2-node1:7000,dc2-node2:7000"
replication_mode = "multi-master"
conflict_resolution = "last-write-wins"  # or "custom"
EOF

# DC2 Configuration
cat >> /etc/rusty-db/rusty-db.conf << 'EOF'
cluster_enabled = true
cluster_name = "global-cluster"
node_name = "dc2-node1"
cluster_members = "dc1-node1:7000,dc1-node2:7000,dc2-node1:7000,dc2-node2:7000"
replication_mode = "multi-master"
conflict_resolution = "last-write-wins"
EOF

# Configure WAN optimization
cat >> /etc/rusty-db/rusty-db.conf << 'EOF'
# Compression for cross-DC traffic
replication_compression = true
replication_compression_algorithm = "lz4"

# Bandwidth throttling
replication_bandwidth_mbps = 100

# Connection pooling
replication_connection_pool_size = 10
EOF
```

---

## Container Deployment

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  rustydb-primary:
    image: rustydb/rusty-db:latest
    container_name: rustydb-primary
    environment:
      - RUSTYDB_MODE=primary
      - RUSTYDB_PASSWORD=secure_password
      - RUSTYDB_REPLICATION_PASSWORD=repl_password
    ports:
      - "5432:5432"
    volumes:
      - rustydb-data:/var/lib/rusty-db
      - rustydb-logs:/var/log/rusty-db
      - ./rusty-db.conf:/etc/rusty-db/rusty-db.conf:ro
    networks:
      - rustydb-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "rusty-db-cli", "--command", "SELECT 1"]
      interval: 10s
      timeout: 5s
      retries: 5

  rustydb-standby:
    image: rustydb/rusty-db:latest
    container_name: rustydb-standby
    environment:
      - RUSTYDB_MODE=standby
      - RUSTYDB_PRIMARY_HOST=rustydb-primary
      - RUSTYDB_PRIMARY_PORT=5432
      - RUSTYDB_REPLICATION_USER=replicator
      - RUSTYDB_REPLICATION_PASSWORD=repl_password
    ports:
      - "5433:5432"
    volumes:
      - rustydb-standby-data:/var/lib/rusty-db
      - rustydb-standby-logs:/var/log/rusty-db
    networks:
      - rustydb-network
    restart: unless-stopped
    depends_on:
      - rustydb-primary

  pgadmin:
    image: dpage/pgadmin4:latest
    container_name: pgadmin
    environment:
      - PGADMIN_DEFAULT_EMAIL=admin@company.com
      - PGADMIN_DEFAULT_PASSWORD=admin_password
    ports:
      - "8080:80"
    networks:
      - rustydb-network
    restart: unless-stopped

volumes:
  rustydb-data:
  rustydb-logs:
  rustydb-standby-data:
  rustydb-standby-logs:

networks:
  rustydb-network:
    driver: bridge
```

### Deploy with Docker Compose

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f rustydb-primary

# Check status
docker-compose ps

# Stop services
docker-compose down

# Stop and remove volumes (⚠️ DATA LOSS)
docker-compose down -v
```

---

## Kubernetes Deployment

### StatefulSet Deployment

```yaml
# rustydb-statefulset.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: rustydb

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: rustydb-config
  namespace: rustydb
data:
  rusty-db.conf: |
    port = 5432
    listen_addresses = "0.0.0.0"
    max_connections = 500
    buffer_pool_size_mb = 8192
    data_directory = "/var/lib/rusty-db/data"
    log_directory = "/var/log/rusty-db"
    replication_mode = "async"

---
apiVersion: v1
kind: Secret
metadata:
  name: rustydb-secrets
  namespace: rustydb
type: Opaque
stringData:
  admin-password: "secure_admin_password"
  replication-password: "secure_repl_password"

---
apiVersion: v1
kind: Service
metadata:
  name: rustydb
  namespace: rustydb
  labels:
    app: rustydb
spec:
  ports:
  - port: 5432
    name: database
  clusterIP: None
  selector:
    app: rustydb

---
apiVersion: v1
kind: Service
metadata:
  name: rustydb-loadbalancer
  namespace: rustydb
spec:
  selector:
    app: rustydb
    role: primary
  ports:
  - protocol: TCP
    port: 5432
    targetPort: 5432
  type: LoadBalancer

---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: rustydb
  namespace: rustydb
spec:
  serviceName: rustydb
  replicas: 3
  selector:
    matchLabels:
      app: rustydb
  template:
    metadata:
      labels:
        app: rustydb
    spec:
      containers:
      - name: rustydb
        image: rustydb/rusty-db:latest
        ports:
        - containerPort: 5432
          name: database
        env:
        - name: RUSTYDB_PASSWORD
          valueFrom:
            secretKeyRef:
              name: rustydb-secrets
              key: admin-password
        - name: RUSTYDB_REPLICATION_PASSWORD
          valueFrom:
            secretKeyRef:
              name: rustydb-secrets
              key: replication-password
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        volumeMounts:
        - name: data
          mountPath: /var/lib/rusty-db
        - name: config
          mountPath: /etc/rusty-db
          readOnly: true
        resources:
          requests:
            memory: "16Gi"
            cpu: "4"
          limits:
            memory: "32Gi"
            cpu: "8"
        livenessProbe:
          exec:
            command:
            - rusty-db-cli
            - --command
            - "SELECT 1"
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          exec:
            command:
            - rusty-db-cli
            - --command
            - "SELECT 1"
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: rustydb-config
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: [ "ReadWriteOnce" ]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 100Gi
```

### Deploy to Kubernetes

```bash
# Create namespace and resources
kubectl apply -f rustydb-statefulset.yaml

# Check pod status
kubectl get pods -n rustydb -w

# Check StatefulSet
kubectl get statefulset -n rustydb

# Check services
kubectl get svc -n rustydb

# View logs
kubectl logs -n rustydb rustydb-0 -f

# Execute commands in pod
kubectl exec -n rustydb rustydb-0 -- rusty-db-cli --command "SELECT * FROM v$cluster_status;"

# Scale StatefulSet
kubectl scale statefulset rustydb -n rustydb --replicas=5

# Delete deployment
kubectl delete -f rustydb-statefulset.yaml
```

### Helm Chart Deployment

```bash
# Add RustyDB Helm repository
helm repo add rustydb https://charts.rustydb.io
helm repo update

# Create values file
cat > values.yaml << 'EOF'
replicaCount: 3

image:
  repository: rustydb/rusty-db
  tag: latest
  pullPolicy: IfNotPresent

resources:
  requests:
    memory: "16Gi"
    cpu: "4"
  limits:
    memory: "32Gi"
    cpu: "8"

persistence:
  enabled: true
  storageClass: "fast-ssd"
  size: 100Gi

replication:
  enabled: true
  mode: async

backup:
  enabled: true
  schedule: "0 2 * * *"
  retention: 30

monitoring:
  enabled: true
  prometheus: true
  grafana: true
EOF

# Install chart
helm install rustydb rustydb/rustydb \
  --namespace rustydb \
  --create-namespace \
  --values values.yaml

# Upgrade chart
helm upgrade rustydb rustydb/rustydb \
  --namespace rustydb \
  --values values.yaml

# Uninstall chart
helm uninstall rustydb --namespace rustydb
```

---

## Cloud Deployments

### AWS Deployment

#### EC2 Deployment

```bash
# Launch EC2 instance with Terraform
cat > main.tf << 'EOF'
provider "aws" {
  region = "us-east-1"
}

# Security Group
resource "aws_security_group" "rustydb" {
  name        = "rustydb-sg"
  description = "Security group for RustyDB"

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]
  }

  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["YOUR_IP/32"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

# EBS Volume for data
resource "aws_ebs_volume" "rustydb_data" {
  availability_zone = "us-east-1a"
  size             = 500
  type             = "gp3"
  iops             = 16000
  throughput       = 1000
  encrypted        = true

  tags = {
    Name = "rustydb-data"
  }
}

# EC2 Instance
resource "aws_instance" "rustydb" {
  ami           = "ami-0c55b159cbfafe1f0"  # Ubuntu 22.04 LTS
  instance_type = "m5.4xlarge"

  root_block_device {
    volume_size = 50
    volume_type = "gp3"
    encrypted   = true
  }

  vpc_security_group_ids = [aws_security_group.rustydb.id]

  user_data = <<-EOF
    #!/bin/bash
    # Install RustyDB
    curl -fsSL https://packages.rustydb.io/install.sh | bash

    # Mount EBS volume
    mkfs.ext4 /dev/xvdf
    mkdir -p /var/lib/rusty-db
    mount /dev/xvdf /var/lib/rusty-db
    echo "/dev/xvdf /var/lib/rusty-db ext4 defaults,nofail 0 2" >> /etc/fstab

    # Initialize database
    sudo -u rustydb rusty-db-server --init
    systemctl enable rusty-db
    systemctl start rusty-db
  EOF

  tags = {
    Name = "rustydb-primary"
  }
}

# Attach EBS volume
resource "aws_volume_attachment" "rustydb_data_att" {
  device_name = "/dev/xvdf"
  volume_id   = aws_ebs_volume.rustydb_data.id
  instance_id = aws_instance.rustydb.id
}

output "instance_public_ip" {
  value = aws_instance.rustydb.public_ip
}
EOF

# Deploy
terraform init
terraform plan
terraform apply
```

#### RDS Custom (Alternative)

```bash
# Create RDS Custom for PostgreSQL and install RustyDB
# (Requires RDS Custom support for custom engines)

# Or use EC2 with automated backups to S3
cat >> /etc/cron.daily/rustydb-backup << 'EOF'
#!/bin/bash
BACKUP_FILE="/tmp/rustydb-backup-$(date +%Y%m%d).backup"
rusty-db-backup --type full --output $BACKUP_FILE --compress --encrypt
aws s3 cp $BACKUP_FILE s3://company-rustydb-backups/
rm $BACKUP_FILE
EOF

chmod +x /etc/cron.daily/rustydb-backup
```

### Google Cloud Platform (GCP)

```bash
# Create Compute Engine instance with Terraform
cat > main.tf << 'EOF'
provider "google" {
  project = "your-project-id"
  region  = "us-central1"
}

resource "google_compute_disk" "rustydb_data" {
  name  = "rustydb-data-disk"
  type  = "pd-ssd"
  zone  = "us-central1-a"
  size  = 500

  physical_block_size_bytes = 4096
}

resource "google_compute_instance" "rustydb" {
  name         = "rustydb-primary"
  machine_type = "n2-standard-16"
  zone         = "us-central1-a"

  boot_disk {
    initialize_params {
      image = "ubuntu-os-cloud/ubuntu-2204-lts"
      size  = 50
      type  = "pd-ssd"
    }
  }

  attached_disk {
    source      = google_compute_disk.rustydb_data.id
    device_name = "data"
  }

  network_interface {
    network = "default"
    access_config {}
  }

  metadata_startup_script = <<-EOF
    #!/bin/bash
    curl -fsSL https://packages.rustydb.io/install.sh | bash
    mkfs.ext4 /dev/sdb
    mkdir -p /var/lib/rusty-db
    mount /dev/sdb /var/lib/rusty-db
    sudo -u rustydb rusty-db-server --init
    systemctl enable rusty-db
    systemctl start rusty-db
  EOF

  tags = ["rustydb"]
}

resource "google_compute_firewall" "rustydb" {
  name    = "rustydb-firewall"
  network = "default"

  allow {
    protocol = "tcp"
    ports    = ["5432"]
  }

  source_ranges = ["10.0.0.0/8"]
  target_tags   = ["rustydb"]
}
EOF

# Deploy
terraform init
terraform apply
```

### Azure Deployment

```bash
# Create Azure VM with Terraform
cat > main.tf << 'EOF'
provider "azurerm" {
  features {}
}

resource "azurerm_resource_group" "rustydb" {
  name     = "rustydb-rg"
  location = "East US"
}

resource "azurerm_virtual_network" "rustydb" {
  name                = "rustydb-vnet"
  address_space       = ["10.0.0.0/16"]
  location            = azurerm_resource_group.rustydb.location
  resource_group_name = azurerm_resource_group.rustydb.name
}

resource "azurerm_subnet" "rustydb" {
  name                 = "rustydb-subnet"
  resource_group_name  = azurerm_resource_group.rustydb.name
  virtual_network_name = azurerm_virtual_network.rustydb.name
  address_prefixes     = ["10.0.1.0/24"]
}

resource "azurerm_network_security_group" "rustydb" {
  name                = "rustydb-nsg"
  location            = azurerm_resource_group.rustydb.location
  resource_group_name = azurerm_resource_group.rustydb.name

  security_rule {
    name                       = "RustyDB"
    priority                   = 1001
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "5432"
    source_address_prefix      = "10.0.0.0/8"
    destination_address_prefix = "*"
  }
}

resource "azurerm_linux_virtual_machine" "rustydb" {
  name                = "rustydb-vm"
  resource_group_name = azurerm_resource_group.rustydb.name
  location            = azurerm_resource_group.rustydb.location
  size                = "Standard_D16s_v3"
  admin_username      = "azureuser"

  admin_ssh_key {
    username   = "azureuser"
    public_key = file("~/.ssh/id_rsa.pub")
  }

  os_disk {
    caching              = "ReadWrite"
    storage_account_type = "Premium_LRS"
    disk_size_gb         = 50
  }

  source_image_reference {
    publisher = "Canonical"
    offer     = "0001-com-ubuntu-server-jammy"
    sku       = "22_04-lts"
    version   = "latest"
  }

  custom_data = base64encode(<<-EOF
    #!/bin/bash
    curl -fsSL https://packages.rustydb.io/install.sh | bash
    sudo -u rustydb rusty-db-server --init
    systemctl enable rusty-db
    systemctl start rusty-db
  EOF
  )
}

resource "azurerm_managed_disk" "rustydb_data" {
  name                 = "rustydb-data-disk"
  location             = azurerm_resource_group.rustydb.location
  resource_group_name  = azurerm_resource_group.rustydb.name
  storage_account_type = "Premium_LRS"
  create_option        = "Empty"
  disk_size_gb         = 500
}

resource "azurerm_virtual_machine_data_disk_attachment" "rustydb" {
  managed_disk_id    = azurerm_managed_disk.rustydb_data.id
  virtual_machine_id = azurerm_linux_virtual_machine.rustydb.id
  lun                = "10"
  caching            = "ReadWrite"
}
EOF

# Deploy
terraform init
terraform apply
```

---

## Scaling Strategies

### Vertical Scaling (Scale Up)

```bash
# 1. Schedule maintenance window
# 2. Create backup
rusty-db-backup --type full --output /backups/pre-scale-backup.backup

# 3. Stop database
systemctl stop rusty-db

# 4. Resize infrastructure (cloud-specific)
# AWS:
aws ec2 modify-instance-attribute \
  --instance-id i-1234567890abcdef0 \
  --instance-type "{\"Value\": \"m5.8xlarge\"}"

# 5. Update configuration
sudo nano /etc/rusty-db/rusty-db.conf
# buffer_pool_size_mb = 32768  # Increase from 8192
# max_connections = 1000        # Increase from 500
# parallel_workers = 16         # Increase from 4

# 6. Start database
systemctl start rusty-db

# 7. Verify performance
rusty-db-cli --command "SELECT * FROM v$resource_usage;"
```

### Horizontal Scaling (Scale Out)

#### Read Replicas

```bash
# Add read replica for read scaling
# Follow standby setup from High Availability section

# Configure application to use read replica for queries
# Connection string for writes: primary:5432
# Connection string for reads: standby1:5432,standby2:5432
```

#### Sharding

```bash
# Enable sharding
rusty-db-cli --command "
ALTER SYSTEM SET sharding_enabled = true;
ALTER SYSTEM SET shard_key_column = 'customer_id';
ALTER SYSTEM SET shard_count = 4;
"

# Create shard nodes
# Node 1: Shard 0-24
# Node 2: Shard 25-49
# Node 3: Shard 50-74
# Node 4: Shard 75-99

# Redistribute data (can take hours for large datasets)
rusty-db-cli --command "REDISTRIBUTE SHARDS;"

# Application changes required:
# - Use consistent hashing for routing
# - Handle cross-shard transactions carefully
```

---

## Upgrade Procedures

### Minor Version Upgrade (e.g., 1.2.3 → 1.2.4)

```bash
# 1. Read release notes
curl https://rustydb.io/releases/1.2.4/notes.txt

# 2. Create backup
rusty-db-backup --type full --output /backups/pre-upgrade-1.2.4.backup

# 3. Update package
# Ubuntu/Debian:
sudo apt-get update
sudo apt-get install rusty-db

# RHEL/CentOS:
sudo yum update rusty-db

# 4. Restart service
sudo systemctl restart rusty-db

# 5. Verify version
rusty-db-cli --command "SELECT version();"

# 6. Check for issues
tail -100 /var/log/rusty-db/alert.log
```

### Major Version Upgrade (e.g., 1.x → 2.x)

```bash
# 1. Read upgrade guide
curl https://rustydb.io/upgrade/1.x-to-2.x.txt

# 2. Test upgrade in staging environment first!

# 3. Schedule maintenance window (expect downtime)

# 4. Create full backup
rusty-db-backup --type full \
  --output /backups/pre-major-upgrade-2.0.backup \
  --verify

# 5. Export schema and data
rusty-db-export --all --output /backups/export-1.x.sql

# 6. Stop old version
systemctl stop rusty-db

# 7. Install new version
# Ubuntu/Debian:
sudo apt-get install rusty-db=2.0.0

# 8. Run upgrade script
rusty-db-upgrade --from 1.x --to 2.0 \
  --data-dir /var/lib/rusty-db \
  --config /etc/rusty-db/rusty-db.conf

# 9. Update configuration (check for deprecated options)
rusty-db-config-check /etc/rusty-db/rusty-db.conf

# 10. Start new version
systemctl start rusty-db

# 11. Verify upgrade
rusty-db-cli --command "SELECT version();"
rusty-db-cli --command "SELECT * FROM v$database_health;"

# 12. Run post-upgrade tasks
rusty-db-cli --command "ANALYZE SCHEMA public;"
rusty-db-cli --command "REINDEX SCHEMA public;"

# 13. Monitor for 24-48 hours
tail -f /var/log/rusty-db/alert.log
```

### Rolling Upgrade (Zero Downtime)

```bash
# For multi-node clusters
# Upgrade standby nodes first, then failover and upgrade old primary

# 1. Upgrade standby-1
# On standby-1:
systemctl stop rusty-db
apt-get install rusty-db=2.0.0
systemctl start rusty-db

# Verify replication still working:
rusty-db-cli --host primary --command "SELECT * FROM v$replication_status;"

# 2. Upgrade standby-2
# Repeat step 1 for standby-2

# 3. Failover to upgraded standby
rusty-db-cluster --switchover --from primary --to standby-1

# 4. Upgrade old primary (now standby)
# On old primary:
systemctl stop rusty-db
apt-get install rusty-db=2.0.0
systemctl start rusty-db

# 5. Verify all nodes on same version
rusty-db-cli --command "SELECT node_name, version FROM v$cluster_nodes;"
```

---

## Rollback Procedures

### Rollback from Failed Upgrade

```bash
# 1. Stop new version
systemctl stop rusty-db

# 2. Downgrade package
# Ubuntu/Debian:
sudo apt-get install rusty-db=1.2.3

# RHEL/CentOS:
sudo yum downgrade rusty-db-1.2.3

# 3. Restore configuration backup
cp /etc/rusty-db/rusty-db.conf.backup /etc/rusty-db/rusty-db.conf

# 4. Restore data (if necessary)
rusty-db-restore --input /backups/pre-upgrade-1.2.4.backup \
  --data-dir /var/lib/rusty-db \
  --force

# 5. Start old version
systemctl start rusty-db

# 6. Verify rollback
rusty-db-cli --command "SELECT version();"
rusty-db-cli --command "SELECT * FROM v$database_health;"
```

### Point-in-Time Recovery

```bash
# Restore to specific point in time
rusty-db-restore --input /backups/pitr-base.backup \
  --archive-dir /var/lib/rusty-db/archive \
  --recovery-target-time "2025-12-11 10:30:00" \
  --data-dir /var/lib/rusty-db

# Start database in recovery mode
systemctl start rusty-db

# Verify recovery target
rusty-db-cli --command "SELECT * FROM v$recovery_status;"

# Promote to normal operation
rusty-db-cli --command "SELECT pg_wal_replay_resume();"
```

---

## Post-Deployment Validation

### Validation Checklist

```bash
# 1. Version check
rusty-db-cli --command "SELECT version();"

# 2. Database health
rusty-db-cli --command "SELECT * FROM v$database_health;"

# 3. Connectivity test
rusty-db-cli --command "SELECT 1;"

# 4. Authentication test
rusty-db-cli --user appuser --password --command "SELECT current_user;"

# 5. Replication status (if applicable)
rusty-db-cli --command "SELECT * FROM v$replication_status;"

# 6. Cluster status (if applicable)
rusty-db-cli --command "SELECT * FROM v$cluster_status;"

# 7. SSL/TLS verification
openssl s_client -connect localhost:5432 -starttls postgres

# 8. Performance baseline
rusty-db-cli --command "EXPLAIN ANALYZE SELECT COUNT(*) FROM pg_class;"

# 9. Backup verification
rusty-db-backup --verify /backups/latest.backup

# 10. Monitoring integration
curl http://localhost:9090/metrics

# 11. Log rotation
logrotate -d /etc/logrotate.d/rusty-db

# 12. Resource usage
rusty-db-cli --command "SELECT * FROM v$resource_usage;"
```

### Performance Benchmarking

```bash
# Install pgbench (compatible with PostgreSQL wire protocol)
sudo apt-get install postgresql-contrib

# Initialize test database
rusty-db-cli --command "CREATE DATABASE benchmark;"
pgbench -i -s 100 benchmark

# Run benchmark
pgbench -c 10 -j 2 -t 10000 benchmark

# Results:
# - TPS (transactions per second)
# - Latency (average, p95, p99)
# - Connection overhead
```

### Security Validation

```bash
# 1. Verify SSL/TLS is enforced
rusty-db-cli --command "SHOW ssl;"

# 2. Test failed authentication
rusty-db-cli --user invalid --password wrong

# 3. Verify audit logging
tail -10 /var/log/rusty-db/audit.log

# 4. Check encryption at rest
rusty-db-cli --command "SELECT * FROM v$encryption_status;"

# 5. Review firewall rules
sudo ufw status
sudo iptables -L -n

# 6. Test privilege separation
rusty-db-cli --user appuser --command "CREATE USER test WITH PASSWORD 'test';"
# Should fail if appuser doesn't have CREATEUSER privilege
```

---

## Appendix

### Quick Reference Commands

```bash
# Service Management
systemctl start rusty-db
systemctl stop rusty-db
systemctl restart rusty-db
systemctl status rusty-db
systemctl enable rusty-db
systemctl disable rusty-db

# Database Client
rusty-db-cli
rusty-db-cli --host remote --port 5432
rusty-db-cli --user admin --password
rusty-db-cli --command "SELECT 1;"
rusty-db-cli --file script.sql

# Backup/Restore
rusty-db-backup --type full --output backup.backup
rusty-db-backup --type incremental --output incr.backup
rusty-db-restore --input backup.backup
rusty-db-backup --verify backup.backup

# Cluster Management
rusty-db-cluster --status
rusty-db-cluster --add-node --name node2
rusty-db-cluster --remove-node --name node2
rusty-db-cluster --failover --target standby-1

# Configuration
rusty-db-config --show
rusty-db-config --set max_connections=1000
rusty-db-config --reload
```

### Troubleshooting Guide

See [OPERATIONS_GUIDE.md](OPERATIONS_GUIDE.md#troubleshooting) for detailed troubleshooting procedures.

### Support Resources

- **Documentation**: https://docs.rustydb.io
- **Community Forum**: https://community.rustydb.io
- **GitHub Issues**: https://github.com/rustydb/rusty-db/issues
- **Enterprise Support**: support@rustydb.io
- **Security Issues**: security@rustydb.io

---

**Document Maintained By**: Infrastructure Team
**Contact**: infra@company.com
**Last Review**: 2025-12-11
**Next Review**: 2026-01-11
