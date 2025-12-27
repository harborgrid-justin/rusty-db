# RustyDB v0.5.1 Quick Start Guide

**Version**: 0.5.1
**Release Date**: December 25, 2025
**Estimated Setup Time**: 15-30 minutes

---

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Basic Configuration](#basic-configuration)
5. [Starting the Server](#starting-the-server)
6. [Verification Procedures](#verification-procedures)
7. [First Steps](#first-steps)
8. [Common Operations](#common-operations)
9. [Troubleshooting](#troubleshooting)
10. [Next Steps](#next-steps)

---

## Introduction

Welcome to RustyDB v0.5.1, an enterprise-grade database management system built entirely in Rust. This guide will walk you through installing, configuring, and running RustyDB for the first time.

**What You'll Learn**:
- How to install RustyDB from source
- How to configure basic database settings
- How to start and stop the database server
- How to verify your installation
- How to execute your first transactions

**Time Required**: 15-30 minutes

---

## Prerequisites

### System Requirements

#### Minimum Requirements
- **CPU**: 2 cores (x86_64 or ARM64)
- **RAM**: 2 GB
- **Disk**: 1 GB free space
- **OS**: Linux, macOS, or Windows

#### Recommended for Production
- **CPU**: 8+ cores
- **RAM**: 16+ GB
- **Disk**: 100+ GB SSD/NVMe
- **OS**: Linux (Ubuntu 20.04+, RHEL 8+, or similar)

### Software Prerequisites

#### Required

**Rust Toolchain** (1.70 or higher):
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

**Git**:
```bash
# Debian/Ubuntu
sudo apt-get install git

# macOS
brew install git

# Windows
# Download from https://git-scm.com/
```

#### Optional

**Development Tools**:
```bash
# Install useful development tools
cargo install cargo-watch      # Auto-rebuild on changes
cargo install cargo-audit       # Security vulnerability scanning
cargo install cargo-flamegraph  # Performance profiling
```

**Database Tools**:
```bash
# PostgreSQL client (for wire protocol compatibility)
sudo apt-get install postgresql-client  # Debian/Ubuntu
brew install postgresql                  # macOS
```

### Network Ports

RustyDB uses the following default ports:
- **5432**: Database server (PostgreSQL wire protocol)
- **8080**: GraphQL API and HTTP endpoints

**Firewall Configuration**:
```bash
# Allow database port (example for ufw)
sudo ufw allow 5432/tcp
sudo ufw allow 8080/tcp
```

---

## Installation

### Option 1: Install from Pre-Built Binary (Recommended - Fastest)

RustyDB v0.5.1 includes pre-built, production-ready binaries.

#### Binary Information

**Linux (x86_64)**:
- Location: `/home/user/rusty-db/builds/linux/`
- Server: `rusty-db-server` (38 MB)
- CLI: `rusty-db-cli` (922 KB)

**Windows (x86_64)**:
- Location: `/home/user/rusty-db/builds/windows/`
- Server: `rusty-db-server.exe` (41 MB)
- CLI: `rusty-db-cli.exe` (876 KB)

**Build Details**:
- Rust Version: 1.92.0
- Build Date: December 25, 2025
- Optimization: Full release optimizations with LTO
- SIMD: Enabled

#### Installation Steps

**Linux**:
```bash
# Navigate to builds directory
cd /home/user/rusty-db/builds/linux/

# Verify the binary
ls -lh rusty-db-server
# Expected: -rwxr-xr-x ... 38M ... rusty-db-server

# Test the binary
./rusty-db-server --version
# Expected: RustyDB v0.5.1 - Enterprise Edition

# Optional: Install to system location
sudo mkdir -p /opt/rustydb/bin
sudo cp rusty-db-server /opt/rustydb/bin/
sudo cp rusty-db-cli /opt/rustydb/bin/
sudo ln -s /opt/rustydb/bin/rusty-db-server /usr/local/bin/rusty-db-server
```

**Windows**:
```powershell
# Navigate to builds directory
cd \home\user\rusty-db\builds\windows\

# Test the binary
.\rusty-db-server.exe --version
# Expected: RustyDB v0.5.1 - Enterprise Edition

# Optional: Install to Program Files
New-Item -ItemType Directory -Path "C:\Program Files\RustyDB\bin" -Force
Copy-Item rusty-db-server.exe "C:\Program Files\RustyDB\bin\"
Copy-Item rusty-db-cli.exe "C:\Program Files\RustyDB\bin\"
```

### Option 2: Build from Source (Alternative)

**Prerequisites**: Rust 1.70+ required

#### Step 1: Clone and Build

```bash
# Clone the repository
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db

# Checkout the v0.5.1 release
git checkout v0.5.1

# Build release binaries (optimized)
cargo build --release
# Build time: 5-15 minutes (first build)

# Verify build
./target/release/rusty-db-server --version
# Expected: RustyDB v0.5.1 - Enterprise Edition

# Check binary sizes
ls -lh target/release/rusty-db-server
# Expected: ~38M
```

### Installation Verification

Run the test suite to verify installation:

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test transaction::    # Transaction tests
cargo test security::       # Security tests
cargo test storage::        # Storage tests
```

Expected output:
```
running 1247 tests
test result: ok. 1247 passed; 0 failed; 0 ignored

   Doc-tests rustydb
running 37 tests
test result: ok. 37 passed; 0 failed; 0 ignored
```

---

## Basic Configuration

### Default Configuration

RustyDB v0.5.1 uses sensible defaults suitable for development:

```
RustyDB v0.5.1 Default Configuration:
    data_directory: "./data"
    page_size: 8192 bytes         // 8 KB pages
    buffer_pool_size: 1000 pages  // ~8 MB buffer pool (1000 × 8KB)
    server_port: 5432             // PostgreSQL wire protocol
    api_port: 8080                // REST/GraphQL API
    max_connections: 100          // Concurrent connections
```

**Binary Build Information**:
- See: `/home/user/rusty-db/builds/BUILD_INFO.md`
- Linux binary: 38 MB (builds/linux/rusty-db-server)
- Windows binary: 41 MB (builds/windows/rusty-db-server.exe)

### Custom Configuration

Create a configuration file (future feature in v0.6.0):

```toml
# config.toml (planned for v0.6.0)
[database]
data_directory = "/var/lib/rustydb"
page_size = 8192                # 8 KB pages (default)

[memory]
buffer_pool_size = 10000        # ~80 MB (10000 pages × 8 KB)
shared_buffers = "1GB"

[network]
server_port = 5432
graphql_port = 8080
max_connections = 200

[security]
enable_tls = true
require_authentication = true
enable_audit_logging = true

[performance]
enable_simd = true
parallel_workers = 8
```

### Environment Variables

Configure via environment variables (v0.5.1):

```bash
# Set data directory
export RUSTYDB_DATA_DIR=/var/lib/rustydb

# Set server port
export RUSTYDB_PORT=5432

# Set GraphQL port
export RUSTYDB_GRAPHQL_PORT=8080

# Set buffer pool size
export RUSTYDB_BUFFER_POOL_SIZE=10000
```

### Directory Structure

RustyDB will create the following directory structure:

```
data/
├── base/                    # Database data files
│   ├── table_1.dat
│   ├── table_1_idx_1.dat
│   └── ...
├── wal/                     # Write-Ahead Log segments
│   ├── 000000010000000000000001
│   ├── 000000010000000000000002
│   └── ...
├── pg_control              # Cluster control file
└── logs/                   # Server logs
    ├── rustydb.log
    └── security_audit.log
```

**Create Data Directory**:
```bash
# Create data directory with proper permissions
sudo mkdir -p /var/lib/rustydb
sudo chown $USER:$USER /var/lib/rustydb
chmod 700 /var/lib/rustydb
```

---

## Starting the Server

### Start in Development Mode

**Option 1: Using Pre-Built Binary**:
```bash
# From builds directory (Linux)
cd /home/user/rusty-db/builds/linux/
./rusty-db-server

# Or if installed to system location
/opt/rustydb/bin/rusty-db-server
rusty-db-server  # If symlinked

# Start with verbose logging
RUST_LOG=debug ./rusty-db-server

# Windows
cd \home\user\rusty-db\builds\windows\
.\rusty-db-server.exe
```

**Option 2: Using Cargo** (if built from source):
```bash
# From repository root
cd /home/user/rusty-db

# Start with cargo (debug build)
cargo run --bin rusty-db-server

# Or use release build
cargo run --release --bin rusty-db-server

# Or run built binary directly
./target/release/rusty-db-server
```

### Start in Production Mode

**Using systemd** (Linux):

**Option 1: Use Pre-Configured Service File** (Recommended):
```bash
# Copy the provided service file
sudo cp /home/user/rusty-db/deploy/systemd/rustydb-single.service /etc/systemd/system/rustydb.service

# Create rustydb user if not exists
sudo useradd -r -s /bin/false -d /var/lib/rustydb rustydb

# Create data directories
sudo mkdir -p /var/lib/rustydb/default
sudo chown -R rustydb:rustydb /var/lib/rustydb

# Install binary
sudo mkdir -p /opt/rustydb/current/bin
sudo cp /home/user/rusty-db/builds/linux/rusty-db-server /opt/rustydb/current/bin/

# Reload and start
sudo systemctl daemon-reload
sudo systemctl enable --now rustydb

# Check status
sudo systemctl status rustydb

# View logs
sudo journalctl -u rustydb -f
```

**Option 2: Create Custom Service File**:

See `/home/user/rusty-db/deploy/systemd/README.md` for detailed instructions.

**Windows Service**:
```powershell
# Use provided Windows service scripts
cd \home\user\rusty-db\deploy\windows\

# Install as service
.\install-service.bat

# Start service
.\start-service.bat

# Check service status (in Services.msc or)
Get-Service RustyDB
```

### Server Startup Sequence

When RustyDB starts, you'll see:

```
[INFO] RustyDB v0.5.1 starting...
[INFO] Initializing storage layer...
[INFO] Buffer pool initialized: 1000 pages (~8 MB)
[INFO] WAL recovery starting...
[INFO] WAL recovery complete: 0 transactions recovered
[INFO] Transaction manager initialized
[INFO] Security modules loaded: 17 modules active
[INFO] GraphQL API server listening on http://0.0.0.0:8080
[INFO] Database server listening on 0.0.0.0:5432
[INFO] RustyDB ready to accept connections
```

### Stop the Server

```bash
# Graceful shutdown (Ctrl+C in terminal)
^C
[INFO] Shutdown signal received
[INFO] Stopping GraphQL server...
[INFO] Closing active connections...
[INFO] Flushing dirty pages...
[INFO] Checkpoint complete
[INFO] RustyDB shutdown complete

# Or using systemd
sudo systemctl stop rustydb

# Or send SIGTERM
kill -TERM $(pidof rusty-db-server)
```

---

## Verification Procedures

### Health Check

**HTTP Health Endpoint**:
```bash
# Check server health
curl http://localhost:8080/health

# Expected response
{
  "status": "healthy",
  "version": "0.5.1",
  "uptime": 120,
  "connections": {
    "active": 0,
    "max": 100
  },
  "buffer_pool": {
    "size": 1000,
    "used": 42,
    "hit_rate": 0.95
  }
}
```

### GraphQL Playground

Open your browser and navigate to:
```
http://localhost:8080/graphql
```

You should see the GraphQL Playground interface with:
- Schema explorer
- Query editor
- Documentation browser

### Test Transaction API

**Query the Server Info**:
```graphql
query {
  serverInfo {
    version
    uptime
    activeConnections
    totalTransactions
  }
}
```

**Begin a Test Transaction**:
```graphql
mutation {
  beginTransaction(isolationLevel: READ_COMMITTED) {
    transactionId
    status
    timestamp
  }
}
```

Expected response:
```json
{
  "data": {
    "beginTransaction": {
      "transactionId": "550e8400-e29b-41d4-a716-446655440000",
      "status": "ACTIVE",
      "timestamp": "2025-12-25T12:34:56.789123456Z"
    }
  }
}
```

### Test Connection Pooling

```bash
# Test concurrent connections
for i in {1..10}; do
  curl http://localhost:8080/health &
done
wait

# Check active connections
curl http://localhost:8080/health | jq '.connections'
```

### Run Integration Tests

```bash
# Run integration test suite
cargo test --test integration

# Run specific integration test
cargo test --test integration test_transaction_lifecycle
```

---

## First Steps

### 1. Create Your First Table (Coming Soon)

**Note**: SQL DDL execution is currently in development. Use GraphQL API for v0.5.1.

Future SQL support:
```sql
CREATE TABLE users (
    id INT PRIMARY KEY,
    username VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 2. Execute Your First Transaction

**Using GraphQL API**:

```graphql
# Step 1: Begin transaction
mutation BeginTxn {
  beginTransaction(isolationLevel: SERIALIZABLE) {
    transactionId
    status
  }
}

# Step 2: Execute operations (example - implementation depends on table creation)
mutation ExecuteTxn {
  executeTransaction(
    operations: [
      {
        operationType: INSERT
        table: "users"
        data: {
          id: 1
          username: "alice"
          email: "alice@example.com"
        }
      }
    ]
    isolationLevel: READ_COMMITTED
  ) {
    success
    executionTimeMs
    affectedRows
  }
}

# Step 3: Commit transaction
mutation CommitTxn {
  commitTransaction(transactionId: "your-transaction-id") {
    success
    commitTimestamp
  }
}
```

### 3. Query Data (Coming Soon)

Future query support:
```graphql
query GetUsers {
  query(
    sql: "SELECT * FROM users WHERE username = $1"
    params: ["alice"]
  ) {
    columns
    rows
    executionTimeMs
  }
}
```

### 4. Explore Security Features

**Check Active Security Modules**:
```graphql
query {
  securityStatus {
    activeModules {
      name
      status
      healthScore
    }
    threatLevel
    complianceScore
  }
}
```

### 5. Monitor Performance

**Access Monitoring Dashboard**:
```bash
# Metrics endpoint (Prometheus-compatible)
curl http://localhost:8080/metrics

# Example output
rustydb_transactions_total{status="committed"} 1234
rustydb_transactions_total{status="aborted"} 5
rustydb_buffer_pool_hit_rate 0.95
rustydb_active_connections 3
```

---

## Common Operations

### Backup and Restore

**Create Backup** (feature in development):
```bash
# Full backup
./target/release/rusty-db-server backup --full --output backup_20251225.tar.gz

# Incremental backup
./target/release/rusty-db-server backup --incremental --output backup_incr_20251225.tar.gz
```

**Restore from Backup**:
```bash
# Stop server first
sudo systemctl stop rustydb

# Restore backup
./target/release/rusty-db-server restore --input backup_20251225.tar.gz

# Start server
sudo systemctl start rustydb
```

### View Logs

**Application Logs**:
```bash
# View server logs
tail -f data/logs/rustydb.log

# View with journalctl (systemd)
sudo journalctl -u rustydb -f

# Filter by level
sudo journalctl -u rustydb -p err
```

**Security Audit Logs**:
```bash
# View security audit log
tail -f data/logs/security_audit.log

# Search for specific events
grep "AUTHENTICATION_FAILURE" data/logs/security_audit.log
```

### Manage Connections

**List Active Connections** (via GraphQL):
```graphql
query {
  activeConnections {
    connectionId
    user
    database
    clientAddress
    connectedSince
    state
  }
}
```

**Terminate Connection**:
```graphql
mutation {
  terminateConnection(connectionId: "conn-123") {
    success
    reason
  }
}
```

### Monitor Transactions

**View Active Transactions**:
```graphql
query {
  activeTransactions {
    transactionId
    startTime
    isolationLevel
    status
    operationCount
    locksHeld
  }
}
```

---

## Troubleshooting

### Server Won't Start

**Problem**: Server fails to start

**Solutions**:

1. **Check port availability**:
```bash
# Check if ports are in use
sudo lsof -i :5432
sudo lsof -i :8080

# Kill conflicting processes
sudo kill -9 <PID>
```

2. **Check data directory permissions**:
```bash
# Verify permissions
ls -ld data/
# Should show: drwx------ (700)

# Fix permissions
chmod 700 data/
```

3. **Check disk space**:
```bash
df -h

# Clean up if needed
cargo clean
rm -rf data/logs/*.old
```

4. **Check logs for errors**:
```bash
# View error logs
tail -n 100 data/logs/rustydb.log | grep ERROR

# View systemd errors
sudo journalctl -u rustydb -p err --since "10 minutes ago"
```

### Connection Refused

**Problem**: Cannot connect to server

**Solutions**:

1. **Verify server is running**:
```bash
# Check process
ps aux | grep rusty-db-server

# Check with systemd
sudo systemctl status rustydb
```

2. **Check network connectivity**:
```bash
# Test GraphQL port
telnet localhost 8080

# Test database port
telnet localhost 5432
```

3. **Check firewall**:
```bash
# Temporarily disable firewall (testing only)
sudo ufw disable

# Or add rules
sudo ufw allow 5432/tcp
sudo ufw allow 8080/tcp
```

### Slow Performance

**Problem**: Queries are slow

**Solutions**:

1. **Increase buffer pool size**:
```bash
# Set larger buffer pool
export RUSTYDB_BUFFER_POOL_SIZE=10000
./target/release/rusty-db-server
```

2. **Check buffer pool hit rate**:
```bash
curl http://localhost:8080/health | jq '.buffer_pool.hit_rate'
# Should be > 0.90 (90%)
```

3. **Enable SIMD acceleration**:
```bash
# Build with SIMD support
cargo build --release --features simd
```

4. **Monitor system resources**:
```bash
# Check CPU and memory
htop

# Check disk I/O
iostat -x 1
```

### Transaction Deadlocks

**Problem**: Transactions deadlocking

**Solutions**:

1. **Check deadlock detection**:
```graphql
query {
  transactionDeadlocks(limit: 10) {
    timestamp
    transactionIds
    conflictingResources
    victimTransactionId
  }
}
```

2. **Review lock acquisition order**:
   - Always acquire locks in the same order
   - Use shorter transactions
   - Consider lower isolation levels

3. **Monitor lock waits**:
```graphql
query {
  lockWaits {
    waitingTransactionId
    blockingTransactionId
    resourceId
    waitDuration
  }
}
```

### Build Errors

**Problem**: Compilation fails

**Solutions**:

1. **Update Rust toolchain**:
```bash
rustup update stable
```

2. **Clean build artifacts**:
```bash
cargo clean
cargo build --release
```

3. **Check dependency versions**:
```bash
cargo update
```

4. **Verify Rust version**:
```bash
rustc --version
# Should be >= 1.70
```

### Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| "Address already in use" | Port 5432 or 8080 taken | Change port or kill process |
| "Permission denied" | Insufficient permissions | Run with sudo or fix permissions |
| "Cannot find data directory" | Missing data directory | Create directory: `mkdir -p data` |
| "WAL recovery failed" | Corrupted WAL files | Restore from backup |
| "Buffer pool exhausted" | Too many concurrent queries | Increase buffer pool size |

---

## Next Steps

### Learning Resources

#### Documentation
- **[Release Notes](./RELEASE_NOTES.md)** - What's new in v0.5.1
- **[Architecture Guide](../../docs/ARCHITECTURE.md)** - System architecture deep dive
- **[Security Architecture](../../docs/SECURITY_ARCHITECTURE.md)** - Security features
- **[Development Guide](../../docs/DEVELOPMENT.md)** - Contributing to RustyDB
- **[Documentation Index](./INDEX.md)** - Complete documentation map

#### Tutorials (Coming Soon)
- Creating tables and indexes
- Writing complex queries
- Implementing stored procedures
- Setting up replication
- Configuring security policies
- Performance tuning

#### Example Applications (Coming Soon)
- Simple CRUD application
- Real-time analytics dashboard
- Multi-tenant SaaS application
- Geospatial application
- Machine learning pipeline

### Advanced Configuration

#### Enable Security Features
```toml
# config.toml (v0.6.0)
[security]
enable_memory_hardening = true
enable_insider_threat_detection = true
enable_network_hardening = true
enable_injection_prevention = true
enable_auto_recovery = true

[security.encryption]
algorithm = "AES-256-GCM"
enable_tde = true
key_rotation_days = 90
```

#### Performance Tuning
```toml
# config.toml (v0.6.0)
[performance]
enable_simd = true
parallel_workers = 8
query_cache_size = "256MB"
enable_jit = true

[buffer_pool]
size = "4GB"
eviction_policy = "ARC"
prefetch_enabled = true
```

#### High Availability
```toml
# config.toml (v0.6.0)
[replication]
mode = "synchronous"
replicas = ["replica1:5432", "replica2:5432"]
enable_auto_failover = true

[clustering]
enable_rac = true
cache_fusion_enabled = true
nodes = ["node1:5432", "node2:5432", "node3:5432"]
```

### Community and Support

#### Get Help
- **GitHub Issues**: https://github.com/harborgrid-justin/rusty-db/issues
- **Discussions**: https://github.com/harborgrid-justin/rusty-db/discussions
- **Documentation**: Full documentation in `/docs` directory

#### Contribute
- Read [DEVELOPMENT.md](../../docs/DEVELOPMENT.md)
- Check open issues for good first issues
- Submit pull requests
- Improve documentation
- Report bugs

#### Stay Updated
- Watch the GitHub repository
- Follow release announcements
- Read the changelog
- Subscribe to security advisories

### Production Deployment Checklist

Before deploying to production:

- [ ] Run full test suite: `cargo test`
- [ ] Build in release mode: `cargo build --release`
- [ ] Configure production settings (ports, data directory)
- [ ] Set up systemd service for auto-start
- [ ] Configure firewall rules
- [ ] Enable security modules
- [ ] Set up backup procedures
- [ ] Configure monitoring and alerting
- [ ] Test disaster recovery procedures
- [ ] Enable TLS/SSL
- [ ] Configure authentication and RBAC
- [ ] Set up audit logging
- [ ] Review security policies
- [ ] Perform load testing
- [ ] Document configuration
- [ ] Train operations team

---

## Summary

Congratulations! You've successfully installed and configured RustyDB v0.5.1.

**What You've Learned**:
- ✅ Installing RustyDB from source
- ✅ Configuring basic database settings
- ✅ Starting and stopping the server
- ✅ Verifying the installation
- ✅ Using the GraphQL API
- ✅ Troubleshooting common issues

**Next Steps**:
1. Explore the [Architecture Guide](../../docs/ARCHITECTURE.md) to understand RustyDB's design
2. Review [Security Architecture](../../docs/SECURITY_ARCHITECTURE.md) for enterprise security
3. Read [Release Notes](./RELEASE_NOTES.md) for detailed feature information
4. Join the community and contribute to development

### Quick Reference Commands

```bash
# Build
cargo build --release

# Run server
./target/release/rusty-db-server

# Run tests
cargo test

# Health check
curl http://localhost:8080/health

# GraphQL playground
open http://localhost:8080/graphql

# View logs
tail -f data/logs/rustydb.log

# Stop server
kill -TERM $(pidof rusty-db-server)
```

### Support

For questions or issues:
- Check [Troubleshooting](#troubleshooting) section
- Review [Documentation Index](./INDEX.md)
- Open a GitHub issue
- Consult the community discussions

---

**RustyDB v0.5.1 - Enterprise Edition**

Built with Rust for Safety, Performance, and Reliability

Happy databasing! 🦀
